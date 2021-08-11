use std::time::Duration;

use async_graphql::Context;
use async_std::task;
use futures::Stream;

pub struct Subscription;

#[Subscription]
impl Subscription {
    async fn hello_world<'ctx>(
        &self,
        _ctx: &'ctx Context<'_>,
    ) -> impl Stream<Item = String> + 'ctx {
        async_stream::stream! {
            loop {
                yield "Hello World!".to_owned();
                task::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}
