import { AnnotationMode, getDocument, GlobalWorkerOptions, normalizeUnicode, TextLayer } from './pdf.min.mjs';

// The preview draws each page and lays the page's text over it, so that the text can be
// selected, copied and searched. It never installs PDF actions, links, annotation layers,
// form controls or a scripting sandbox.
GlobalWorkerOptions.workerSrc = new URL('./pdf.worker.min.mjs', import.meta.url).href;
const local = path => new URL(path, import.meta.url).href;
const $ = id => document.getElementById(id);
const view = $('view');
const pages = $('pages');
const status = $('status');

// What the reader says. The app serves these in the person's language; without it, English.
const text = {
    loading: 'Loading PDF…',
    failed: 'This PDF cannot be displayed.',
    page: 'Page {n}',
    zoomIn: 'Zoom in',
    zoomOut: 'Zoom out',
    fit: 'Fit width',
    find: 'Find in document',
    previous: 'Previous match',
    next: 'Next match',
    none: 'No matches',
};
try {
    const served = await (await fetch(local('./strings.json'), { cache: 'no-store' })).json();
    for (const key of Object.keys(text)) {
        if (typeof served[key] === 'string' && served[key]) text[key] = served[key];
    }
    if (typeof served.lang === 'string' && served.lang) document.documentElement.lang = served.lang;
} catch (_) {
    // The texts above stay.
}
for (const [id, label] of [
    ['out', text.zoomOut], ['in', text.zoomIn], ['fit', text.fit], ['prev', text.previous], ['next', text.next],
]) {
    $(id).title = label;
    $(id).setAttribute('aria-label', label);
}
$('find').placeholder = text.find;
$('find').setAttribute('aria-label', text.find);
status.textContent = text.loading;

const POINT = 96 / 72; // CSS pixels of one PDF unit at 100%
const STOPS = [25, 33, 50, 67, 75, 90, 100, 110, 125, 150, 175, 200, 250, 300, 400, 500];
const LEAST = STOPS[0] / 100 * POINT;
const MOST = STOPS[STOPS.length - 1] / 100 * POINT;
const MAX_PIXELS = 8_000_000; // of one page's picture
const MAX_SIDE = 4096;
const MAX_HITS = 5000;
const clamp = (value, least, most) => Math.min(Math.max(value, least), most);
// A PDF made on a Mac often names a common Han character by the radical that looks like it,
// and no search for the character finds it. What is copied holds the character that is meant.
const meant = picked => picked.replace(/[\u2E80-\u2FDF\uF900-\uFAFF]/g, sign => sign.normalize('NFKC'));
const clean = picked => meant(normalizeUnicode(picked)).replaceAll('\0', '');
// For finding, fullwidth forms count as the plain ones too. Only a sign that becomes one other
// sign is replaced, so that every place in a page's text stays where it is.
const plain = read => read.replace(/[\u2E80-\u2FDF\uF900-\uFAFF\uFF00-\uFFEF]/g, sign => {
    const other = sign.normalize('NFKC');
    return other.length === 1 ? other : sign;
});

// Selection over a text layer's separately placed pieces, after PDF.js's own viewer: an
// element that ends each layer follows the selection's moving end, so that a drag that
// leaves the text does not jump to the start of the page.
const layers = new Map(); // a text layer -> the element that ends it
let pointerDown = false;
let lastRange = null;
function settle(end, layer) {
    layer.append(end);
    end.style.width = '';
    end.style.height = '';
    layer.classList.remove('selecting');
}
document.addEventListener('pointerdown', () => { pointerDown = true; });
document.addEventListener('pointerup', () => { pointerDown = false; layers.forEach(settle); });
window.addEventListener('blur', () => { pointerDown = false; layers.forEach(settle); });
document.addEventListener('keyup', () => { if (!pointerDown) layers.forEach(settle); });
document.addEventListener('selectionchange', () => {
    const selection = document.getSelection();
    if (!selection || selection.rangeCount === 0) {
        layers.forEach(settle);
        return;
    }
    const active = new Set();
    for (let i = 0; i < selection.rangeCount; ++i) {
        const range = selection.getRangeAt(i);
        for (const layer of layers.keys()) {
            if (!active.has(layer) && range.intersectsNode(layer)) active.add(layer);
        }
    }
    for (const [layer, end] of layers) {
        if (active.has(layer)) layer.classList.add('selecting');
        else settle(end, layer);
    }
    try {
        const range = selection.getRangeAt(0);
        const fromStart = lastRange && (
            range.compareBoundaryPoints(Range.END_TO_END, lastRange) === 0
            || range.compareBoundaryPoints(Range.START_TO_END, lastRange) === 0);
        let anchor = fromStart ? range.startContainer : range.endContainer;
        if (anchor.nodeType === Node.TEXT_NODE) anchor = anchor.parentNode;
        if (!fromStart && range.endOffset === 0) {
            do {
                while (!anchor.previousSibling) anchor = anchor.parentNode;
                anchor = anchor.previousSibling;
            } while (!anchor.childNodes.length);
        }
        const layer = anchor.parentElement?.closest('.textLayer');
        const end = layers.get(layer);
        if (end) {
            end.style.width = layer.style.width;
            end.style.height = layer.style.height;
            // WebKit keeps the prefixed property apart from the plain one, and obeys it.
            end.style.userSelect = end.style.webkitUserSelect = 'text';
            anchor.parentElement.insertBefore(end, fromStart ? anchor : anchor.nextSibling);
        }
        lastRange = range.cloneRange();
    } catch (_) {
        // A selection that ends outside the pages needs no help.
    }
});
// What is copied is the text as it reads, not the glyph forms a page was set in.
document.addEventListener('copy', event => {
    if (event.target instanceof Element && event.target.closest('#bar')) return;
    const picked = String(document.getSelection() ?? '');
    if (!picked || !event.clipboardData) return;
    event.clipboardData.setData('text/plain', clean(picked));
    event.preventDefault();
});

try {
    const response = await fetch(local('./document.pdf'), { cache: 'no-store' });
    if (!response.ok) throw new Error('PDF unavailable');
    const pdf = await getDocument({
        data: new Uint8Array(await response.arrayBuffer()),
        cMapUrl: local('./cmaps/'),
        cMapPacked: true,
        standardFontDataUrl: local('./standard_fonts/'),
        wasmUrl: local('./decoders/'),
        enableXfa: false,
        isEvalSupported: false,
        useWasm: false,
        maxImageSize: 16_000_000,
        canvasMaxAreaInBytes: 32 * 1024 * 1024,
    }).promise;
    if (!pdf.numPages || pdf.numPages > 500) throw new Error('Page limit');
    const first = (await pdf.getPage(1)).getViewport({ scale: 1 });
    if (!(first.width > 0 && first.height > 0)) throw new Error('Invalid page geometry');

    // ---- pages and their size

    let fit = true; // the pages follow the width of the pane
    let scale = 1; // CSS pixels of one PDF unit
    let generation = 1; // a new one whenever the pages change their size on screen
    let drawing = null; // the picture being made
    const slots = [];
    const bySection = new Map();
    const near = new Set();

    const fitScale = () => clamp(Math.min(Math.max(view.clientWidth - 24, 320), 1100) / first.width, LEAST, MOST);

    function size(slot) {
        slot.section.style.width = `${Math.floor(slot.width * scale)}px`;
        slot.section.style.height = `${Math.floor(slot.height * scale)}px`;
        slot.section.style.setProperty('--total-scale-factor', scale * slot.unit);
    }

    /// The page on show at the top of the pane, and how far into it the pane starts.
    function place() {
        const top = view.scrollTop;
        let low = 0;
        let high = slots.length - 1;
        while (low < high) {
            const middle = (low + high) >> 1;
            const section = slots[middle].section;
            if (section.offsetTop + section.offsetHeight > top) high = middle;
            else low = middle + 1;
        }
        const section = slots[low].section;
        return {
            slot: slots[low],
            part: (top - section.offsetTop) / Math.max(section.offsetHeight, 1),
            across: (view.scrollLeft + view.clientWidth / 2) / Math.max(view.scrollWidth, 1),
        };
    }

    function shown() {
        $('zoom').textContent = `${Math.round(scale / POINT * 100)}%`;
        $('out').disabled = scale <= LEAST * 1.001;
        $('in').disabled = scale >= MOST * 0.999;
        $('fit').setAttribute('aria-pressed', String(fit));
    }

    function redraw() {
        ++generation;
        drawing?.cancel();
        renderNearby();
    }

    function resize(next) {
        next = clamp(next, LEAST, MOST);
        if (Math.abs(next - scale) > scale * 0.004) {
            const held = place();
            scale = next;
            for (const slot of slots) size(slot);
            view.scrollTop = held.slot.section.offsetTop + held.part * held.slot.section.offsetHeight;
            view.scrollLeft = held.across * view.scrollWidth - view.clientWidth / 2;
            redraw();
        }
        shown();
    }

    function step(direction) {
        const now = scale / POINT * 100;
        const stop = direction > 0
            ? STOPS.find(value => value > now * 1.01) ?? STOPS[STOPS.length - 1]
            : [...STOPS].reverse().find(value => value < now * 0.99) ?? STOPS[0];
        fit = false;
        resize(stop / 100 * POINT);
    }

    function fitWidth() {
        fit = true;
        resize(fitScale());
    }

    // ---- drawing the pages near the pane

    function release(slot) {
        const canvas = slot.section.querySelector('canvas');
        // Drop the backing storage of a picture far outside the pane.
        if (canvas) canvas.width = canvas.height = 1;
        if (slot.layer) layers.delete(slot.layer);
        slot.section.replaceChildren();
        slot.drawn = 0;
        slot.layer = slot.spans = slot.marks = null;
    }

    function note(slot) {
        release(slot);
        const message = document.createElement('span');
        message.className = 'note';
        message.textContent = text.failed;
        slot.section.append(message);
        slot.drawn = generation;
    }

    async function draw(slot) {
        const mine = generation;
        let page;
        try {
            page = await pdf.getPage(slot.number);
            const one = page.getViewport({ scale: 1 });
            if (!(one.width > 0 && one.height > 0)) throw new Error('Invalid page geometry');
            if (!slot.known) {
                const raw = one.rawDims;
                slot.known = true;
                slot.width = one.width;
                slot.height = one.height;
                slot.unit = one.width / (one.rotation % 180 === 0 ? raw.pageWidth : raw.pageHeight);
                size(slot);
            }
            const viewport = page.getViewport({ scale });
            // The screen's own pixels, as far as a few pages near the pane stay affordable.
            const dense = Math.min(
                clamp(window.devicePixelRatio || 1, 1, 3),
                MAX_SIDE / viewport.width, MAX_SIDE / viewport.height,
                Math.sqrt(MAX_PIXELS / (viewport.width * viewport.height)),
            );
            const canvas = document.createElement('canvas');
            canvas.width = Math.max(1, Math.round(viewport.width * dense));
            canvas.height = Math.max(1, Math.round(viewport.height * dense));
            canvas.setAttribute('role', 'img');
            canvas.setAttribute('aria-label', text.page.replace('{n}', slot.number));
            drawing = page.render({
                canvas,
                viewport,
                transform: [canvas.width / viewport.width, 0, 0, canvas.height / viewport.height, 0, 0],
                annotationMode: AnnotationMode.DISABLE,
            });
            // The native WebView may suspend animation frames while the tab is detached.
            drawing.onContinue = resume => setTimeout(resume, 0);
            await drawing.promise;

            let layer = null;
            let spans = null;
            try {
                const content = await page.getTextContent();
                remember(slot, content.items);
                layer = document.createElement('div');
                layer.className = 'textLayer';
                // PDF.js measures each piece in the document's own language, which chooses the
                // font; the page's language is the reader's and must not choose another here.
                layer.lang = content.lang || '';
                const laid = new TextLayer({ textContentSource: content, container: layer, viewport });
                await laid.render();
                if (laid.textDivs.length === content.items.length) spans = laid.textDivs;
            } catch (_) {
                // A page whose text cannot be read is still shown.
                layer = null;
            }
            if (mine !== generation || !near.has(slot)) {
                canvas.width = canvas.height = 1;
                return;
            }
            release(slot);
            const marks = document.createElement('div');
            marks.className = 'hits';
            slot.section.append(canvas, marks);
            if (layer) {
                const end = document.createElement('div');
                end.className = 'endOfContent';
                layer.append(end);
                layer.addEventListener('mousedown', () => layer.classList.add('selecting'));
                layers.set(layer, end);
                slot.section.append(layer);
            }
            slot.layer = layer;
            slot.spans = spans;
            slot.marks = marks;
            slot.drawn = mine;
            status.hidden = true;
            paint(slot);
        } catch (_) {
            // A newer size cancels the picture being made; the page is drawn again at that size.
            if (mine === generation && near.has(slot)) {
                note(slot);
                status.hidden = true;
            }
        } finally {
            drawing = null;
            page?.cleanup();
        }
    }

    let busy = false;
    let again = false;
    async function renderNearby() {
        if (busy) {
            again = true;
            return;
        }
        busy = true;
        try {
            do {
                again = false;
                for (const slot of [...near].sort((a, b) => a.number - b.number)) {
                    if (near.has(slot) && slot.drawn !== generation) await draw(slot);
                }
            } while (again);
        } finally {
            busy = false;
        }
    }

    // ---- finding text

    const found = { query: '', hits: [], at: -1, run: 0, seek: false };

    /// Keeps a page's text as one string, with where each of its pieces starts in it. A line's
    /// end reads as a space, except after a character of a script that is written without spaces.
    function remember(slot, items) {
        if (slot.text !== undefined) return;
        const starts = new Int32Array(items.length);
        let all = '';
        items.forEach((item, index) => {
            starts[index] = all.length;
            all += item.str;
            if (item.hasEOL && all.length) {
                const last = all.charCodeAt(all.length - 1);
                const unspaced = (last >= 0x2E80 && last <= 0x9FFF) || (last >= 0xF900 && last <= 0xFAFF)
                    || (last >= 0xFF00 && last <= 0xFFEF);
                if (!unspaced) all += '\n';
            }
        });
        slot.text = plain(all);
        slot.starts = starts;
    }

    async function read(slot) {
        if (slot.text !== undefined) return;
        let page;
        try {
            page = await pdf.getPage(slot.number);
            remember(slot, (await page.getTextContent()).items);
        } catch (_) {
            slot.text = '';
            slot.starts = new Int32Array(0);
        } finally {
            page?.cleanup();
        }
    }

    /// The piece of a page's text that holds the character at `offset`.
    function piece(slot, offset) {
        const starts = slot.starts;
        let low = 0;
        let high = starts.length - 1;
        while (low < high) {
            const middle = (low + high + 1) >> 1;
            if (starts[middle] <= offset) low = middle;
            else high = middle - 1;
        }
        return low;
    }

    function tally() {
        const count = found.hits.length;
        $('count').textContent = !found.query ? '' : count ? `${found.at + 1} / ${count}` : text.none;
        $('prev').disabled = $('next').disabled = count < 2;
    }

    /// Marks what was found on a drawn page, as boxes under its text.
    function paint(slot) {
        const marks = slot.marks;
        if (!marks) return;
        marks.replaceChildren();
        if (!slot.spans || !found.hits.length) return;
        const frame = slot.section.getBoundingClientRect();
        if (!(frame.width > 0 && frame.height > 0)) return;
        let current = null;
        let boxes = 0;
        found.hits.forEach((hit, index) => {
            if (hit.slot !== slot || boxes > 4000) return;
            const from = piece(slot, hit.start);
            const to = piece(slot, hit.end - 1);
            for (let at = from; at <= to; ++at) {
                const node = slot.spans[at]?.firstChild;
                if (!node || node.nodeType !== Node.TEXT_NODE || !node.isConnected) continue;
                const range = document.createRange();
                range.setStart(node, clamp(hit.start - slot.starts[at], 0, node.length));
                range.setEnd(node, clamp(hit.end - slot.starts[at], 0, node.length));
                for (const box of range.getClientRects()) {
                    if (box.width < 0.5 || box.height < 0.5) continue;
                    const mark = document.createElement('i');
                    mark.style.left = `${(box.left - frame.left) / frame.width * 100}%`;
                    mark.style.top = `${(box.top - frame.top) / frame.height * 100}%`;
                    mark.style.width = `${box.width / frame.width * 100}%`;
                    mark.style.height = `${box.height / frame.height * 100}%`;
                    if (index === found.at) {
                        mark.className = 'now';
                        current ??= mark;
                    }
                    marks.append(mark);
                    ++boxes;
                }
            }
        });
        if (current && found.seek) {
            found.seek = false;
            current.scrollIntoView({ block: 'center', inline: 'nearest' });
        }
    }

    function repaint() {
        for (const slot of near) if (slot.drawn) paint(slot);
    }

    /// Brings the current match on show: at once on a drawn page, else when its page is drawn.
    function reveal() {
        tally();
        const hit = found.hits[found.at];
        if (!hit) {
            repaint();
            return;
        }
        found.seek = true;
        if (!(hit.slot.drawn && near.has(hit.slot))) {
            view.scrollTop = hit.slot.section.offsetTop
                + hit.slot.section.offsetHeight * (hit.start / Math.max(hit.slot.text.length, 1)) - view.clientHeight / 3;
        }
        repaint();
    }

    function move(direction) {
        const count = found.hits.length;
        if (!count) return;
        found.at = (found.at + direction + count) % count;
        reveal();
    }

    async function search(query) {
        query = query.trim();
        if (query === found.query) return;
        const run = ++found.run;
        found.query = query;
        found.hits = [];
        found.at = -1;
        found.seek = false;
        if (!query) {
            tally();
            repaint();
            return;
        }
        $('count').textContent = '…';
        const pattern = new RegExp(
            plain(query).replace(/[.*+?^${}()|[\]\\]/g, '\\$&').replace(/\s+/g, '\\s+'), 'giu');
        const from = place().slot.number;
        const hits = [];
        for (const slot of slots) {
            await read(slot);
            if (run !== found.run) return;
            pattern.lastIndex = 0;
            for (let match; hits.length < MAX_HITS && (match = pattern.exec(slot.text));) {
                if (!match[0].length) {
                    ++pattern.lastIndex;
                    continue;
                }
                hits.push({ slot, start: match.index, end: match.index + match[0].length });
            }
        }
        found.hits = hits;
        // The first match at or after the page on show, as a reader expects.
        found.at = hits.length ? Math.max(hits.findIndex(hit => hit.slot.number >= from), 0) : -1;
        reveal();
    }

    // ---- the pane

    const observer = new IntersectionObserver(entries => {
        for (const entry of entries) {
            const slot = bySection.get(entry.target);
            if (entry.isIntersecting) near.add(slot);
            else if (near.delete(slot)) release(slot);
        }
        renderNearby();
    }, { root: view, rootMargin: '400px' });

    scale = fitScale();
    for (let number = 1; number <= pdf.numPages; ++number) {
        const section = document.createElement('section');
        section.className = 'page';
        section.dataset.page = number;
        const slot = {
            section, number, width: first.width, height: first.height, unit: 1, known: false, drawn: 0,
            layer: null, spans: null, marks: null, text: undefined, starts: null,
        };
        size(slot);
        slots.push(slot);
        bySection.set(section, slot);
        pages.append(section);
        observer.observe(section);
    }

    function where() {
        $('where').textContent = `${place().slot.number} / ${slots.length}`;
    }
    // Another screen has other pixels: the same size is drawn again for them. An engine that
    // does not tell is asked whenever the pane moves.
    let density = window.devicePixelRatio;
    function pixels() {
        if (density === window.devicePixelRatio) return;
        density = window.devicePixelRatio;
        redraw();
    }
    (function told() {
        try {
            matchMedia(`(resolution: ${window.devicePixelRatio}dppx)`).addEventListener('change', () => {
                pixels();
                told();
            }, { once: true });
        } catch (_) {
            // Asked below.
        }
    })();

    let scrolled = 0;
    view.addEventListener('scroll', () => {
        scrolled ||= setTimeout(() => { scrolled = 0; where(); pixels(); }, 80);
    }, { passive: true });

    let resized = 0;
    new ResizeObserver(() => {
        clearTimeout(resized);
        resized = setTimeout(() => {
            if (fit) resize(fitScale());
            where();
            pixels();
        }, 120);
    }).observe(view);

    $('out').addEventListener('click', () => step(-1));
    $('in').addEventListener('click', () => step(1));
    $('fit').addEventListener('click', fitWidth);
    $('prev').addEventListener('click', () => move(-1));
    $('next').addEventListener('click', () => move(1));

    let typed = 0;
    const field = $('find');
    field.addEventListener('input', () => {
        clearTimeout(typed);
        typed = setTimeout(() => search(field.value), 180);
    });
    field.addEventListener('keydown', event => {
        if (event.key === 'Enter' && !event.isComposing) {
            event.preventDefault();
            clearTimeout(typed);
            if (field.value.trim() === found.query) move(event.shiftKey ? -1 : 1);
            else search(field.value);
        }
    });

    document.addEventListener('keydown', event => {
        if (!(event.metaKey || event.ctrlKey) || event.altKey) return;
        const key = event.key.toLowerCase();
        if (key === 'f') {
            field.focus();
            field.select();
        } else if (key === 'g') {
            move(event.shiftKey ? -1 : 1);
        } else if (key === '=' || key === '+') {
            step(1);
        } else if (key === '-') {
            step(-1);
        } else if (key === '0') {
            fitWidth();
        } else if (key === 'c' && !(event.target instanceof Element && event.target.closest('#bar'))) {
            // A WebView inside an app may have no menu that copies; the selection is copied here.
            const picked = String(document.getSelection() ?? '');
            if (!picked) return;
            if (!document.execCommand('copy')) navigator.clipboard?.writeText(clean(picked)).catch(() => {});
        } else {
            return;
        }
        event.preventDefault();
    });

    // A pinch: a wheel with the control key where the system reports it so, a gesture in WebKit.
    let turned = 0;
    view.addEventListener('wheel', event => {
        if (!event.ctrlKey) return;
        event.preventDefault();
        turned += event.deltaY;
        if (Math.abs(turned) < 24) return;
        fit = false;
        resize(scale * (turned < 0 ? 1.1 : 1 / 1.1));
        turned = 0;
    }, { passive: false });
    let pinched = 0;
    view.addEventListener('gesturestart', event => {
        event.preventDefault();
        pinched = scale;
    });
    view.addEventListener('gesturechange', event => {
        event.preventDefault();
        if (!(pinched > 0 && event.scale > 0)) return;
        fit = false;
        if (Math.abs(pinched * event.scale - scale) > scale * 0.05) resize(pinched * event.scale);
    });
    view.addEventListener('gestureend', event => event.preventDefault());

    for (const id of ['in', 'fit', 'find']) $(id).disabled = false;
    shown();
    where();
    view.focus({ preventScroll: true });
    // A WebView that is not on show yet reports no page as near; the first one is drawn anyway.
    near.add(slots[0]);
    await renderNearby();
} catch (_) {
    status.textContent = text.failed;
    status.hidden = false;
}
