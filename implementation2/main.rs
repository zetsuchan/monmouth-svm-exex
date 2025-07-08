//! # SVMExEx
//!
//! This is a prototype Execution Extension for Reth that embeds a minimal Solana VM executor.
//! It allows Monmouth (an EVM L2) to interpret and execute Solana bytecode inside an ExEx context.
//! This enables trustless ETH <-> SOL interactions and agent-native multi-runtime composition.

use futures_util::{FutureExt, TryStreamExt};
use reth::{
    api::FullNodeComponents, builder::NodeTypes, primitives::EthPrimitives,
    providers::BlockIdReader,
};
use reth_exex::ExExContext;
use reth_node_ethereum::EthereumNode;
use reth_tracing::tracing::{info, warn};
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{ready, Context, Poll},
};
use tokio::sync::mpsc;

// Placeholder for Solana runtime types
mod solana_vm {
    pub struct SvmExecutor;

    impl SvmExecutor {
        pub fn new() -> Self {
            Self {}
        }

        pub fn execute_instruction(&self, data: &[u8]) -> anyhow::Result<String> {
            // Placeholder: run the Solana instruction and return result
            Ok(format!("Executed SVM payload: {} bytes", data.len()))
        }
    }
}

use solana_vm::SvmExecutor;

/// SVMExEx handles execution of embedded Solana bytecode in Ethereum txs
struct SVMExEx<Node: FullNodeComponents> {
    ctx: ExExContext<Node>,
    svm: SvmExecutor,
}

impl<Node: FullNodeComponents<Types: NodeTypes<Primitives = EthPrimitives>>> SVMExEx<Node> {
    fn new(ctx: ExExContext<Node>) -> Self {
        Self { ctx, svm: SvmExecutor::new() }
    }
}

impl<Node: FullNodeComponents<Types: NodeTypes<Primitives = EthPrimitives>>> Future
    for SVMExEx<Node>
{
    type Output = eyre::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        while let Some(notification) = ready!(this.ctx.notifications.try_next().poll_unpin(cx))? {
            if let Some(chain) = notification.committed_chain() {
                for (block, txs) in chain.blocks_and_receipts() {
                    for tx in block.body().transactions_iter() {
                        let input = tx.input();

                        if input.starts_with(b"SVM") {
                            let payload = &input[3..]; // strip prefix

                            match this.svm.execute_instruction(payload) {
                                Ok(output) => info!("[SVM-ExEx] Result: {}", output),
                                Err(err) => warn!("[SVM-ExEx] Execution failed: {:?}", err),
                            }
                        }
                    }
                }
            }
        }
        Poll::Ready(Ok(()))
    }
}

fn main() -> eyre::Result<()> {
    reth::cli::Cli::parse_args().run(|builder, _| async move {
        let handle = builder
            .node(EthereumNode::default())
            .install_exex("svm-coprocessor", |ctx| async {
                Ok(SVMExEx::new(ctx))
            })
            .launch()
            .await?;

        handle.wait_for_node_exit().await
    })
}
