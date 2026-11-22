use super::*;
use hagency_agent_local::questions::question_queue;
use hagency_runtime::questions::Answer;

#[tokio::test]
async fn claude_questions_share_host_channel_without_using_tool_permission() {
    let mut f = Fixture::new(Kind::Claude, "questions", ToolPolicy::Deny);
    let mut process = f.process().await;
    process
        .session
        .open_context(&mut f.ledger, &f.scope, &f.profile)
        .await
        .unwrap();
    let (queue, mut approvals) = codex::approval_queue(1).unwrap();
    let (questions, mut incoming) = question_queue();
    let queue = queue.with_questions(questions);
    let run = process.session.run_with_started(
        &mut f.ledger,
        "@owner:test",
        &f.scope,
        "call",
        "dispatch",
        "ask questions",
        BudgetMode::Postpaid,
        &queue,
        None,
    );
    let host = async {
        let pending = incoming.recv().await.unwrap();
        assert_eq!(pending.scope, f.scope);
        assert_eq!(pending.dispatch, "dispatch");
        assert_eq!(pending.questions.len(), 4);
        let reply = pending.reply(
            [
                (
                    "q0".into(),
                    Answer::Options(vec!["One".into(), "Two".into()]),
                ),
                ("q1".into(), Answer::Text("Custom".into())),
                ("q2".into(), Answer::Option("One".into())),
                ("q3".into(), Answer::Option("Two".into())),
            ]
            .into(),
        );
        assert!(reply.accepted().await);
    };
    let (completed, ()) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(run, host)
    })
    .await
    .unwrap();
    assert_eq!(completed.unwrap().text, "offline reply");
    assert!(approvals.try_recv().is_err());
    assert_eq!(f.spent(), (24, 0));
    let answer: serde_json::Value =
        serde_json::from_slice(&fs::read(f.path("question-answer")).unwrap()).unwrap();
    assert_eq!(answer["updatedInput"]["answers"]["Question 0?"], "One, Two");
    assert_eq!(answer["updatedInput"]["answers"]["Question 1?"], "Custom");
    process.stop().await.unwrap();
}

#[tokio::test]
async fn claude_withdrawal_closes_waiting_card_without_replaying_input() {
    let mut f = Fixture::new(Kind::Claude, "question_cancel", ToolPolicy::Deny);
    let mut process = f.process().await;
    process
        .session
        .open_context(&mut f.ledger, &f.scope, &f.profile)
        .await
        .unwrap();
    let (queue, _) = codex::approval_queue(1).unwrap();
    let (questions, mut incoming) = question_queue();
    let queue = queue.with_questions(questions);
    let presented = f.path("question-presented");
    let (started, running) = tokio::sync::oneshot::channel();
    let run = process.session.run_with_started(
        &mut f.ledger,
        "@owner:test",
        &f.scope,
        "call",
        "dispatch",
        "ask questions",
        BudgetMode::Postpaid,
        &queue,
        Some(started),
    );
    let host = async {
        // The host is slow on purpose: the whole thread stops once the turn
        // runs, so the question waits unread while the fixture goes on. A
        // withdrawal that is read before the question was presented sends
        // no card at all, and the wait for the card below would not end.
        // The fixture therefore withdraws only once the marker exists.
        running.await.expect("the turn started");
        std::thread::sleep(std::time::Duration::from_millis(300));
        let pending = incoming.recv().await.unwrap();
        fs::write(presented, "presented").unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while !pending.is_closed() {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert!(!pending.reply(Default::default()).accepted().await);
    };
    let (completed, ()) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(run, host)
    })
    .await
    .unwrap();
    assert!(completed.is_ok());
    assert!(!f.path("question-answer").exists());
    assert_eq!(f.spent(), (24, 0));
    process.stop().await.unwrap();
}
