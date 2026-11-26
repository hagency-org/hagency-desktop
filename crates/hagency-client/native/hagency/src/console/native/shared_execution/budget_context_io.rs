//! Context input on exact v3 work routes; historical descriptors keep their original bytes.
use super::*;
use super::context_io::ContextAction;
use super::budget_work_io::WorkAction;
use super::context_io::paired;
/// The exchanges that bring the context of a plaintext work on the budget
/// routes, asked again as `context_io::context` is. This host's record of the
/// input is `context_io::prepare`, the same for both.
pub(super) async fn context(
    b: &Bound,
    w: &SharedWorkItem,
    authority: SharedActivityRef,
    until: Instant,
) -> Result<(SharedWorkInputMode, SharedContextRecord), NativeError> {
    paired(b).await?;
    let mode: Timed<SharedWorkInputMode> = b
        .upload(
            WorkAction::Context(Box::new(ContextAction::Mode {
                work: w.id.clone(),
                input: SharedWorkEvidenceInput {
                    work_digest: digest(w)?,
                    runtime_command_digest: w.runtime_command_digest.clone(),
                },
            }))
            .into(),
            until,
        )
        .await?;
    mode.value.validate().map_err(|_| invalid())?;
    let mode = mode.value;
    if mode.mode != "thread_context_v1" {
        return Err(failure(409, "native_context_protocol_required"));
    }
    let action = if let Some(hash) = &mode.context_digest {
        ContextAction::Recover {
            work: w.id.clone(),
            input: SharedContextRecoverInput {
                work_digest: digest(w)?,
                runtime_command_digest: w.runtime_command_digest.clone(),
                context_digest: hash.clone(),
            },
        }
    } else {
        ContextAction::Freeze {
            work: w.id.clone(),
            input: SharedContextInput {
                authority,
                work_digest: digest(w)?,
                version: 3,
            },
        }
    };
    let context: Timed<SharedContextRecord> =
        b.upload(WorkAction::Context(Box::new(action)).into(), until).await?;
    Ok((mode, context.value))
}
pub(super) async fn start(
    b: &Bound,
    r: &OwnerResourceRegistry,
    attempt: SharedStartAttempt,
    w: &SharedWorkItem,
    input: &SharedPreparedInput,
    authority: SharedActivityRef,
    until: Instant,
) -> Result<SharedCallCustody, NativeError> {
    if let Some(c) = input.context() {
        let start: Timed<SharedContextStart> = b
            .upload(
                WorkAction::Context(Box::new(ContextAction::Start {
                    work: w.id.clone(),
                    input: SharedContextStartInput {
                        authority,
                        work_digest: digest(w)?,
                        context_digest: c.context_digest.clone(),
                        input_digest: c.input_digest.clone(),
                    },
                }))
                .into(),
                until,
            )
            .await?;
        r.issue_shared_context_call(attempt, start.value)
            .await
            .map_err(store)
    } else {
        if !w.encrypted {
            return Err(failure(409, "native_context_protocol_required"));
        }
        let start: Timed<SharedWorkStart> = b
            .upload(
                WorkAction::Start(SharedWorkInput {
                    authority,
                    work_id: w.id.clone(),
                    work_digest: digest(w)?,
                }).into(),
                until,
            )
            .await?;
        r.issue_shared_call(attempt, start.value)
            .await
            .map_err(store)
    }
}
