use crate::{Match, Mock, Request};
use bastion::prelude::*;
use http_types::{Response, StatusCode};
use log::{debug, warn};

#[derive(Clone)]
pub(crate) struct MockActor {
    pub actor_ref: ChildRef,
}

#[derive(Clone, Debug)]
struct Reset {}

impl MockActor {
    /// Start an instance of our MockActor and return a reference to it.
    pub(crate) fn start() -> MockActor {
        let mock_actors = Bastion::children(|children: Children| {
            children.with_exec(move |ctx: BastionContext| async move {
                let mut mocks: Vec<Mock> = vec![];
                loop {
                    msg! { ctx.recv().await?,
                        _reset: Reset =!> {
                            debug!("Dropping all mocks.");
                            mocks = vec![];
                            answer!(ctx, "Reset.").unwrap();
                        };
                        mock: Mock =!> {
                            debug!("Registering mock.");
                            mocks.push(mock);
                            answer!(ctx, "Registered.").unwrap();
                        };
                        request: http_types::Request =!> {
                            debug!("Handling request.");
                            let request = Request::from(request).await;

                            let mut response: Option<Response> = None;
                            for mock in &mocks {
                                if mock.matches(&request) {
                                    response = Some(mock.response());
                                    break;
                                }
                            }
                            if let Some(response) = response {
                                answer!(ctx, response).unwrap();
                            } else {
                                debug!("Got unexpected request:\n{}", request);
                                let res = Response::new(StatusCode::NotFound);
                                answer!(ctx, res).unwrap();
                            }
                        };
                        _: _ => {
                            warn!("Received a message I was not listening for.");
                        };
                    }
                }
            })
        })
        .expect("Couldn't create the mock actor.");
        // We actually started only one actor
        let mock_actor = mock_actors.elems()[0].clone();
        MockActor {
            actor_ref: mock_actor,
        }
    }

    pub(crate) async fn register(&self, mock: Mock) {
        self.actor_ref.ask_anonymously(mock).unwrap().await.unwrap();
    }

    pub(crate) async fn reset(&self) {
        self.actor_ref
            .ask_anonymously(Reset {})
            .unwrap()
            .await
            .unwrap();
    }
}
