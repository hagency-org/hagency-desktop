import { AnnotationMode, getDocument, GlobalWorkerOptions } from './pdf.min.mjs';

// The preview renders pixels only. It never installs PDF actions, links,
// annotation layers, form controls, a scripting sandbox, or a viewer toolbar.
GlobalWorkerOptions.workerSrc = new URL('./pdf.worker.min.mjs', import.meta.url).href;
const pages = document.querySelector('#pages');
const status = document.querySelector('#status');
const local = path => new URL(path, import.meta.url).href;
const failed = 'This PDF cannot be displayed.';

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
    const ratio = first.width / first.height;
    const maxPixels = 2_400_000;
    const near = new Set();
    let rendering = false;

    async function renderNearby() {
        if (rendering) return;
        rendering = true;
        try {
            for (const slot of near) {
                if (slot.firstChild) continue;
                const number = Number(slot.dataset.page);
                let page;
                try {
                    page = await pdf.getPage(number);
                    const size = page.getViewport({ scale: 1 });
                    if (!(size.width > 0 && size.height > 0)) throw new Error('Invalid page geometry');
                    slot.style.aspectRatio = `${size.width} / ${size.height}`;
                    const scale = Math.min(
                        Math.max(slot.clientWidth, 400) * Math.min(devicePixelRatio, 2) / size.width,
                        2048 / size.width, 2048 / size.height,
                        Math.sqrt(maxPixels / (size.width * size.height)),
                    );
                    const viewport = page.getViewport({ scale });
                    const canvas = document.createElement('canvas');
                    canvas.width = Math.max(1, Math.ceil(viewport.width));
                    canvas.height = Math.max(1, Math.ceil(viewport.height));
                    canvas.setAttribute('role', 'img');
                    canvas.setAttribute('aria-label', `Page ${number}`);
                    const render = page.render({ canvas, viewport, annotationMode: AnnotationMode.DISABLE });
                    // The native WebView may suspend animation frames while the tab is detached.
                    render.onContinue = resume => setTimeout(resume, 0);
                    await render.promise;
                    if (near.has(slot)) slot.append(canvas);
                    status.hidden = true;
                    document.title = `Page ${number}`;
                } catch (_) {
                    if (near.has(slot)) {
                        const message = document.createElement('span');
                        message.textContent = failed;
                        slot.append(message);
                        status.hidden = true;
                    }
                } finally {
                    page?.cleanup();
                }
            }
        } finally {
            rendering = false;
        }
    }

    const observer = new IntersectionObserver(entries => {
        for (const entry of entries) {
            const slot = entry.target;
            if (entry.isIntersecting) near.add(slot);
            else {
                near.delete(slot);
                // Drop backing storage for canvases far outside the viewport.
                if (slot.firstChild instanceof HTMLCanvasElement) {
                    slot.firstChild.width = slot.firstChild.height = 1;
                }
                slot.replaceChildren();
            }
        }
        renderNearby().catch(() => { status.textContent = failed; status.hidden = false; });
    }, { rootMargin: '300px' });

    for (let number = 1; number <= pdf.numPages; ++number) {
        const slot = document.createElement('section');
        slot.className = 'page';
        slot.dataset.page = number;
        slot.style.aspectRatio = String(ratio);
        pages.append(slot);
        observer.observe(slot);
    }
    near.add(pages.firstElementChild);
    await renderNearby();
} catch (_) {
    status.textContent = failed;
    status.hidden = false;
    document.title = failed;
}
