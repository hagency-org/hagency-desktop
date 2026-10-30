//! Load canonical readback before enabling the existing native consent footer.
//! A monthly-maximum amendment has its own sheet in the confirmation family:
//! it stays open while the change saves and then shows what changed, or why
//! nothing did and which amount would work.
use super::*;
use super::sheet_ui::{Fact, Figure, SheetGlyph, SheetTone, SheetView};
use crate::host::hagency::resource_edit::{CeilingFacts, EditRefusal, ResourceEditConsent};
use ::hagency::native_owner::CeilingAmendmentReview;

/// Native panel words for a monthly-maximum amendment: title, purpose,
/// consequence and confirmation. The sheet itself is `ceiling_view`.
pub(super) const AMENDMENT_WORDS: (&str, &str, &str, &str) = (
    "Change the monthly maximum",
    "",
    "",
    "Save monthly maximum",
);

/// The English catalog key shown when the review could not be loaded.
fn failure_text(code: Option<&str>, amendment: bool) -> &'static str {
    match code {
        None if amendment => "Reading this pool…",
        None => "Reading current resource configuration…",
        Some("management_resource_edit_changed") => "This pool just changed, so nothing was saved. Close this window and refresh, then try again.",
        Some("management_resource_edit_not_available") => "This pool can’t be changed here right now. Close this window and refresh to see why.",
        Some("management_ceiling_amendment_not_available") => "This pool’s monthly maximum can’t change right now. Close this window and refresh to see why.",
        Some("management_ceiling_clear_refused") => "A pool already offered to a server can’t drop its monthly maximum. You can lower it instead.",
        Some("management_ceiling_unchanged") => "The monthly maximum already has this value. Nothing was changed.",
        Some("management_ceiling_below_commitments") => "The monthly maximum can’t be lower than what this month has committed: what was used plus what’s given to Agents.",
        Some("management_ceiling_usage_unsettled") => "Some Agent use is still being reported, so the monthly maximum can only go up for now. Keep this computer online, then try again.",
        Some(_) => "The current settings couldn’t be confirmed. Close this window and refresh, then try again.",
    }
}
/// Refusals the mini app also explains beside its maximum control. The
/// sheet replies with the code, so the page says the same thing after it closes.
fn ceiling_refusal(code: &str) -> bool {
    matches!(
        code,
        "management_resource_edit_changed"
            | "management_ceiling_amendment_not_available"
            | "management_ceiling_clear_refused"
            | "management_ceiling_unchanged"
            | "management_ceiling_below_commitments"
            | "management_ceiling_usage_unsettled"
    )
}
fn amount(tokens: Option<u64>) -> String {
    tokens.map(crate::utils::format_token_count).unwrap_or_else(|| crate::i18n::tr("Unknown").into())
}

/// The monthly-maximum sheet's facts, copied from the native prompt so the
/// shipped sheet and its offline preview render through one function.
pub(super) struct CeilingState<'a> {
    pub account: &'a str,
    pub resource_id: &'a str,
    /// The amendment under review.
    pub review: Option<&'a CeilingAmendmentReview>,
    /// The same request was already saved: (previous, saved) maximum.
    pub recorded: Option<(u64, u64)>,
    pub facts: Option<&'a CeilingFacts>,
    /// The review could not be shown, and why.
    pub failure: Option<&'a str>,
    pub saving: bool,
    /// What saving returned: the saved amendment, or why it was refused.
    pub saved: Option<&'a Result<Value, String>>,
    pub more: bool,
}
/// Everything the monthly-maximum sheet shows, from native state only.
pub(super) fn ceiling_view(s: &CeilingState) -> SheetView {
    let tr = crate::i18n::tr;
    let fmt = crate::i18n::format;
    let facts = s.facts.cloned().unwrap_or_default();
    let name = s
        .review
        .and_then(|review| review.name.clone())
        .or_else(|| facts.name.clone())
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty());
    let previous = s.review.map(|review| review.previous_tokens).or(facts.current);
    let proposed = s.review.map(|review| review.tokens).or(facts.proposed);
    let per_request = s.review.and_then(|review| review.per_request_tokens).or(facts.per_request);
    let mut v = SheetView {
        glyph: SheetGlyph::Maximum,
        title: tr("Change the Monthly Maximum").into(),
        pool: name.clone().unwrap_or_default(),
        caption: tr("Monthly maximum").into(),
        more_label: tr("Technical Details").into(),
        more_open: s.more,
        cancel: tr("Cancel").into(),
        primary: tr("Save Monthly Maximum").into(),
        ..SheetView::default()
    };
    let mut technical = vec![(tr("Your account"), s.account.to_owned()), (tr("Resource ID"), s.resource_id.to_owned())];
    if let Some(review) = s.review {
        technical.push((tr("Change number"), review.sequence.to_string()));
        technical.push((tr("Resource revision"), review.revision.clone()));
    }
    v.technical = technical
        .into_iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(label, value)| format!("{label}\n{value}"))
        .collect::<Vec<_>>()
        .join("\n\n");
    // What stays the same: the per-request limit and running Agents.
    let per_request_fact = |done: bool| {
        let fact = Fact::new(
            tr("Per-request limit"),
            match per_request {
                Some(limit) => fmt("{amount} (unchanged)", &[("amount", amount(Some(limit)))]),
                None => tr("Unchanged").into(),
            },
        );
        if done && proposed.zip(previous).is_some_and(|(new, old)| new > old) {
            fact.note(tr("To let each Agent get more at a time, raise it on the pool’s page."))
        } else {
            fact.note(tr("Change it separately on the pool’s page if you need to."))
        }
    };
    let committed_fact = || match facts.committed {
        Some(committed) => Some(
            Fact::new(tr("Committed this month"), amount(Some(committed)))
                .note(tr("Used plus given to Agents. The monthly maximum can’t go below it.")),
        ),
        None if s.facts.is_some() => Some(
            Fact::new(tr("Committed this month"), tr("Still being reported"))
                .note(tr("Until it’s known, the monthly maximum can only go up.")),
        ),
        None => None,
    };
    let saved = s.saved.map(|result| result.as_ref().map_err(String::as_str));
    match (saved, s.saving, s.failure) {
        // Saved: what changed, in the same words as the page's result.
        (Some(Ok(result)), _, _) => {
            let amended = &result["ceilingAmendment"];
            let before = amended["previousTokens"].as_u64().or(previous);
            let after = amended["tokens"].as_u64().or(proposed);
            v.tone = SheetTone::Done;
            v.title = tr("Monthly Maximum Saved").into();
            v.before = before.map(|n| Figure::new(amount(Some(n)), tr("Before")));
            v.after = Some(Figure::new(amount(after), tr("Now")));
            v.lead = fmt("Agents can now use up to {amount} in total this month.", &[("amount", amount(after))]);
            v.facts_label = tr("What stays the same").into();
            v.facts = vec![per_request_fact(true), Fact::new(tr("Running Agents"), tr("Not affected"))];
            v.primary = tr("Done").into();
            v.primary_visible = true;
            v.primary_enabled = true;
            v.primary_closes = true;
            v.cancel.clear();
        }
        // Refused, at review or when saving: why, and the amount that works.
        (Some(Err(code)), _, _) | (None, false, Some(code)) => {
            refusal(&mut v, code, previous, proposed, &facts);
            if let Some(fact) = committed_fact() {
                v.facts_label = tr("This pool").into();
                v.facts = vec![Fact::new(tr("Monthly maximum now"), amount(previous)), fact];
            }
            v.primary = tr("OK").into();
            v.primary_visible = true;
            v.primary_enabled = true;
            v.primary_closes = true;
            v.cancel.clear();
        }
        // Saving: the change being saved stays in view.
        (None, true, _) => {
            v.tone = SheetTone::Working;
            v.before = previous.map(|n| Figure::new(amount(Some(n)), tr("Now")));
            v.after = Some(Figure::new(amount(proposed), tr("Being saved")));
            v.status = sentences(&[tr("Saving…").into(), tr("Closing this window doesn’t cancel it.").into()]);
            v.cancel = tr("Close").into();
        }
        (None, false, None) => match (s.review, s.recorded) {
            (_, Some((before, after))) => {
                v.tone = SheetTone::Done;
                v.title = tr("Already Saved").into();
                v.before = Some(Figure::new(amount(Some(before)), tr("Before")));
                v.after = Some(Figure::new(amount(Some(after)), tr("Now")));
                v.lead = tr("This change was saved earlier. Nothing new is saved.").into();
                v.primary = tr("Done").into();
                v.primary_visible = true;
                v.primary_enabled = true;
                v.cancel.clear();
            }
            (Some(review), None) => {
                v.tone = SheetTone::Proposal;
                v.before = Some(Figure::new(amount(Some(review.previous_tokens)), tr("Now")));
                v.after = Some(Figure::new(amount(Some(review.tokens)), tr("After you confirm")));
                v.lead = if review.tokens > review.previous_tokens {
                    fmt("Agents can use up to {amount} in total this month.", &[("amount", amount(Some(review.tokens)))])
                } else {
                    fmt(
                        "Agents can use up to {amount} in total this month. New requests beyond it can’t be approved.",
                        &[("amount", amount(Some(review.tokens)))],
                    )
                };
                v.facts_label = tr("This pool").into();
                v.facts = vec![per_request_fact(false), Fact::new(tr("Running Agents"), tr("Not affected"))];
                if let Some(fact) = committed_fact() {
                    v.facts.push(fact);
                }
                v.facts.push(Fact::new(tr("Takes effect"), tr("When you confirm")));
                v.primary_visible = true;
                v.primary_enabled = true;
            }
            (None, None) => {
                v.tone = SheetTone::Working;
                v.caption.clear();
                v.technical.clear();
                v.status = tr("Reading this pool…").into();
            }
        },
    }
    v
}
/// A refused maximum: why, and which amount would work where it is known.
fn refusal(v: &mut SheetView, code: &str, previous: Option<u64>, proposed: Option<u64>, facts: &CeilingFacts) {
    let tr = crate::i18n::tr;
    let fmt = crate::i18n::format;
    v.tone = SheetTone::Failed;
    v.title = tr("Monthly Maximum Not Saved").into();
    v.status = tr("Nothing was changed.").into();
    match (code, facts.committed) {
        ("management_ceiling_below_commitments", Some(committed)) => {
            v.caption = tr("The monthly maximum must be at least").into();
            v.after = Some(Figure::new(amount(Some(committed)), tr("Committed this month")));
            v.lead = match proposed {
                Some(chosen) => fmt(
                    "You chose {chosen}, which is less than the {committed} this month has committed. Choose {committed} or more on the pool’s page.",
                    &[("chosen", amount(Some(chosen))), ("committed", amount(Some(committed)))],
                ),
                None => fmt("Choose {committed} or more on the pool’s page.", &[("committed", amount(Some(committed)))]),
            };
        }
        ("management_ceiling_usage_unsettled", _) if previous.is_some() => {
            v.caption = tr("For now it can only go up from").into();
            v.after = Some(Figure::new(amount(previous), tr("Now")));
            v.lead = tr(failure_text(Some(code), true)).into();
        }
        ("management_ceiling_unchanged", _) if previous.is_some() => {
            v.after = Some(Figure::new(amount(previous), tr("Now")));
            v.lead = tr(failure_text(Some(code), true)).into();
            v.status.clear();
        }
        _ => {
            v.caption.clear();
            v.lead = tr(failure_text(Some(code), true)).into();
        }
    }
}
fn sentences(parts: &[String]) -> String {
    let separator = if crate::i18n::language() == crate::i18n::Language::Chinese { "" } else { " " };
    parts.iter().filter(|p| !p.is_empty()).cloned().collect::<Vec<_>>().join(separator)
}

/// Offline review only: the monthly maximum sheet for a fixture, through the
/// same view as the real window. `state`: proposal (default), saving, saved,
/// or refused with `error`. Amounts are tokens.
#[cfg(any(test, feature = "onboarding-test-support"))]
pub(super) fn preview(fixture: &Value) -> Result<SheetView, String> {
    let tokens = |key: &str| fixture[key].as_u64();
    let review = CeilingAmendmentReview {
        resource_id: fixture["resource"].as_str().unwrap_or("resource_original").into(),
        name: fixture["label"].as_str().map(str::to_owned),
        revision: "0".repeat(64),
        sequence: 1,
        previous_tokens: tokens("current").ok_or("preview field current missing")?,
        tokens: tokens("proposed").ok_or("preview field proposed missing")?,
        per_request_tokens: tokens("perRequest"),
    };
    let facts = CeilingFacts {
        name: review.name.clone(),
        current: Some(review.previous_tokens),
        proposed: Some(review.tokens),
        committed: tokens("committed"),
        per_request: review.per_request_tokens,
    };
    let state = fixture["state"].as_str().unwrap_or("proposal");
    let failure = fixture["error"].as_str();
    let saved = match (state, failure) {
        ("saved", None) => Some(Ok(json!({"resourceId": review.resource_id, "ceilingAmendment": {"sequence": 2, "previousTokens": review.previous_tokens, "tokens": review.tokens}}))),
        ("saved", Some(code)) => Some(Err(code.to_owned())),
        _ => None,
    };
    let account = fixture["account"].as_str().unwrap_or_default();
    let view = ceiling_view(&CeilingState {
        account,
        resource_id: &review.resource_id,
        review: if failure.is_some() || state == "loading" { None } else { Some(&review) },
        recorded: None,
        facts: if state == "loading" { None } else { Some(&facts) },
        failure: if state == "saved" { None } else { failure },
        saving: state == "saving",
        saved: saved.as_ref(),
        more: fixture["technical"].as_bool().unwrap_or(false),
    });
    Ok(view)
}

pub(super) struct ResourceEditPrompt {
    account: String,
    /// The request asked for a monthly-maximum amendment.
    amendment: bool,
    expires: Instant,
    receiver: Option<Receiver<Result<ResourceEditConsent, EditRefusal>>>,
    consent: Option<ResourceEditConsent>,
    failure: Option<String>,
    /// Kept after confirming, when the consent itself is used up.
    resource_id: String,
    review: Option<CeilingAmendmentReview>,
    recorded: Option<(u64, u64)>,
    facts: Option<CeilingFacts>,
    /// A confirmed amendment that is saving, then what saving returned.
    saving: Option<Receiver<ServiceEvent>>,
    saved: Option<Result<Value, String>>,
    /// The mini app has its reply for this request.
    replied: bool,
    /// The sheet's single disclosure is open.
    more: bool,
}
impl ResourceEditPrompt {
    pub(super) fn begin(consent: ResourceEditConsent) -> Self {
        let account = consent.account().to_owned();
        let amendment = consent.amendment_requested();
        let resource_id = consent.resource_id().to_owned();
        let (tx, receiver) = mpsc::sync_channel(1);
        crate::sliding_sync::spawn_async_task(async move {
            let result = consent.load().await;
            let _ = tx.try_send(result);
            SignalToUI::set_ui_signal();
        });
        Self {
            account,
            amendment,
            expires: Instant::now() + Duration::from_secs(300),
            receiver: Some(receiver),
            consent: None,
            failure: None,
            resource_id,
            review: None,
            recorded: None,
            facts: None,
            saving: None,
            saved: None,
            replied: false,
            more: false,
        }
    }
    pub(super) fn poll(&mut self) -> bool {
        let mut changed = false;
        if let Some(receiver) = &self.receiver {
            let result = match receiver.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => Some(Err(EditRefusal { code: "management_unavailable".into(), facts: None })),
            };
            if let Some(result) = result {
                self.receiver = None;
                match result {
                    Ok(consent) => {
                        self.review = consent.ceiling_review().cloned();
                        self.recorded = consent.recorded_amendment();
                        self.facts = consent.facts().cloned();
                        self.consent = Some(consent);
                    }
                    Err(refusal) => {
                        self.failure = Some(refusal.code);
                        self.facts = refusal.facts;
                    }
                }
                changed = true;
            }
        }
        if let Some(receiver) = &self.saving {
            let result = match receiver.try_recv() {
                Ok(ServiceEvent::Complete(result)) => Some(result),
                Ok(_) | Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => Some(Err("management_outcome_unknown".into())),
            };
            if let Some(result) = result {
                self.saving = None;
                self.saved = Some(result);
                changed = true;
            }
        }
        changed
    }
    pub(super) fn account(&self) -> &str {
        &self.account
    }
    pub(super) fn expired(&self) -> bool {
        Instant::now() >= self.expires
            || self
                .consent
                .as_ref()
                .is_some_and(ResourceEditConsent::expired)
    }
    pub(super) fn ready(&self) -> bool {
        self.consent.is_some() && !self.expired()
    }
    pub(super) fn recovering(&self) -> bool {
        self.consent
            .as_ref()
            .is_some_and(ResourceEditConsent::recovering)
    }
    pub(super) fn amending(&self) -> bool {
        self.consent
            .as_ref()
            .is_some_and(ResourceEditConsent::amending)
    }
    /// A monthly-maximum request keeps its own window, even when its review
    /// could not be loaded and the window only says why.
    pub(super) fn amendment_requested(&self) -> bool {
        self.amendment
    }
    pub(super) fn details(&self) -> String {
        if let Some(consent) = &self.consent {
            return consent.details();
        }
        crate::i18n::tr(failure_text(self.failure.as_deref(), self.amendment)).into()
    }
    /// The sheet for this monthly-maximum request, from native state only.
    pub(super) fn sheet(&self) -> SheetView {
        ceiling_view(&CeilingState {
            account: &self.account,
            resource_id: &self.resource_id,
            review: self.review.as_ref(),
            recorded: self.recorded,
            facts: self.facts.as_ref(),
            failure: self.failure.as_deref(),
            saving: self.saving.is_some(),
            saved: self.saved.as_ref(),
            more: self.more,
        })
    }
    pub(super) fn toggle_more(&mut self) {
        self.more = !self.more;
    }
    /// Confirm the amendment shown and keep the sheet: it saves, then shows
    /// the result. Only a ready, unrecovered amendment review starts saving.
    pub(super) fn save(&mut self) -> bool {
        if self.saving.is_some() || self.saved.is_some() || !self.ready() || !self.amending() {
            return false;
        }
        let Some(consent) = self.consent.take() else {
            return false;
        };
        let (tx, receiver) = mpsc::sync_channel(1);
        crate::sliding_sync::spawn_async_task(async move {
            let result = consent.accept().await;
            let _ = tx.try_send(ServiceEvent::Complete(result));
            SignalToUI::set_ui_signal();
        });
        // Saving has its own deadline inside the writer; the sheet stays.
        self.expires = Instant::now() + Duration::from_secs(600);
        self.saving = Some(receiver);
        true
    }
    pub(super) fn saving(&self) -> bool {
        self.saving.is_some()
    }
    /// The save's result for the mini app, once, as soon as it is known.
    pub(super) fn take_reply(&mut self) -> Option<Result<Value, String>> {
        if self.replied {
            return None;
        }
        let reply = self.saved.clone()?;
        self.replied = true;
        Some(reply)
    }
    /// Closing the sheet: a save still running keeps its reply path, and a
    /// refusal at review tells the mini app why instead of "cancelled".
    pub(super) fn close(&mut self) -> CeilingClose {
        if self.replied {
            return CeilingClose::Replied;
        }
        if let Some(receiver) = self.saving.take() {
            self.replied = true;
            return CeilingClose::Saving(receiver);
        }
        if let Some(reply) = self.take_reply() {
            return CeilingClose::Reply(reply);
        }
        match self.failure.as_deref() {
            Some(code) if ceiling_refusal(code) => {
                self.replied = true;
                CeilingClose::Reply(Err(code.to_owned()))
            }
            _ => CeilingClose::Cancelled,
        }
    }
    pub(super) async fn accept(self) -> Result<Value, String> {
        if self.expired() {
            return Err("provider_confirmation_expired".into());
        }
        self.consent
            .ok_or_else(|| "resource_edit_review_required".to_owned())?
            .accept()
            .await
    }
}
/// How a closing monthly-maximum sheet answers the mini app.
pub(super) enum CeilingClose {
    /// The result was already sent.
    Replied,
    /// Still saving: the result is sent when it arrives.
    Saving(Receiver<ServiceEvent>),
    /// Send this now.
    Reply(Result<Value, String>),
    /// Nothing was confirmed.
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{Language, text_for};

    #[test]
    fn ceiling_amendment_panel_and_refusals_are_localized_and_distinct() {
        let (title, purpose, consequence, confirm) = AMENDMENT_WORDS;
        for english in [title, confirm] {
            assert_ne!(text_for(Language::Chinese, english), english, "{english}");
        }
        assert!(purpose.is_empty() && consequence.is_empty());
        assert_eq!(text_for(Language::Chinese, title), "修改每月上限");
        assert_eq!(text_for(Language::Chinese, confirm), "保存每月上限");
        let generic = failure_text(Some("management_unavailable"), true);
        for code in [
            "management_ceiling_amendment_not_available",
            "management_ceiling_clear_refused",
            "management_ceiling_unchanged",
            "management_ceiling_below_commitments",
            "management_ceiling_usage_unsettled",
        ] {
            let english = failure_text(Some(code), true);
            assert_ne!(english, generic, "{code} has its own reason");
            assert_ne!(text_for(Language::Chinese, english), english, "{code} is translated");
            assert!(ceiling_refusal(code), "{code} reaches the page as itself");
        }
        assert!(failure_text(Some("management_ceiling_below_commitments"), true).contains("what was used plus what’s given to Agents"));
        assert_eq!(
            text_for(Language::Chinese, failure_text(Some("management_ceiling_usage_unsettled"), true)),
            "部分 Agent 的用量还在上报，所以暂时只能调高、不能调低每月上限。请保持这台电脑在线，稍后再试。"
        );
        assert_eq!(text_for(Language::Chinese, failure_text(None, true)), "正在读取这个资源池…");
    }

    fn fixture(extra: Value) -> Value {
        let mut base = json!({"panel": "maximum", "account": "@owner:test", "label": "Research",
            "current": 1_000_000u64, "proposed": 2_000_000u64, "perRequest": 200_000u64, "committed": 250_000u64});
        for (key, value) in extra.as_object().unwrap() {
            base[key] = value.clone();
        }
        base
    }

    #[test]
    fn the_maximum_sheet_shows_before_and_after_and_what_stays_the_same() {
        let view = preview(&fixture(json!({}))).unwrap();
        assert_eq!(view.tone, SheetTone::Proposal);
        assert_eq!(view.title, "Change the Monthly Maximum");
        assert_eq!(view.pool, "Research");
        assert_eq!(view.before, Some(Figure::new("1 M", "Now")));
        assert_eq!(view.after, Some(Figure::new("2 M", "After you confirm")));
        let rows: Vec<(&str, &str)> = view.facts.iter().map(|f| (f.label.as_str(), f.value.as_str())).collect();
        assert_eq!(rows, [
            ("Per-request limit", "0.2 M (unchanged)"),
            ("Running Agents", "Not affected"),
            ("Committed this month", "0.25 M"),
            ("Takes effect", "When you confirm"),
        ]);
        // The account is labelled and behind the disclosure, never a bare ID.
        for shown in [&view.title, &view.pool, &view.lead, &view.status] {
            assert!(!shown.contains("@owner") && !shown.contains("resource_original"), "{shown}");
        }
        assert!(view.technical.starts_with("Your account\n@owner:test"));
        assert_eq!((view.primary.as_str(), view.cancel.as_str()), ("Save Monthly Maximum", "Cancel"));
        assert!(view.primary_enabled && !view.primary_closes);
    }

    #[test]
    fn a_saved_or_refused_maximum_looks_different_and_says_what_works() {
        let saved = preview(&fixture(json!({"state": "saved"}))).unwrap();
        assert_eq!(saved.tone, SheetTone::Done);
        assert_eq!(saved.title, "Monthly Maximum Saved");
        assert_eq!((saved.before.clone().unwrap().value, saved.after.clone().unwrap().value), ("1 M".into(), "2 M".into()));
        assert!(saved.primary_closes && saved.cancel.is_empty() && saved.primary == "Done");
        let refused = preview(&fixture(json!({"error": "management_ceiling_below_commitments", "proposed": 200_000u64}))).unwrap();
        assert_eq!(refused.tone, SheetTone::Failed);
        assert_eq!(refused.title, "Monthly Maximum Not Saved");
        assert_eq!(refused.after, Some(Figure::new("0.25 M", "Committed this month")));
        assert!(refused.lead.contains("0.2 M") && refused.lead.contains("0.25 M"), "{}", refused.lead);
        assert_eq!((refused.primary.as_str(), refused.cancel.as_str()), ("OK", ""));
        let saving = preview(&fixture(json!({"state": "saving"}))).unwrap();
        assert_eq!(saving.tone, SheetTone::Working);
        assert!(!saving.primary_visible && saving.cancel == "Close");
        assert!(saving.status.contains("doesn’t cancel"), "{}", saving.status);
    }
}
