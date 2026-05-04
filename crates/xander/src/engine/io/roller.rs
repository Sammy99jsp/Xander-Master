use d20::DiceRoller;
use smol::future::FutureExt;
use thiserror::Error;
use xander_runtime::futures::future::LocalBoxFuture;

pub trait Roller: std::fmt::Debug + Send + Sync {
    fn roll<'b, 'a: 'b>(
        &'a self,
        epxr: &'b d20::DExpr,
    ) -> LocalBoxFuture<'b, Result<d20::ValTree, DiceRollerError>>;
}

#[derive(Debug, Error)]
pub enum DiceRollerError {
    #[error("Error in dice expression {0:?}")]
    BadExpression(d20::ValTreeError),

    #[error("Error occurred whilst rolling the dice: {0}")]
    RollerError(String),
}

impl<T> Roller for T
where
    T: DiceRoller + std::fmt::Debug + Send + Sync,
{
    fn roll<'b, 'a: 'b>(
        &'a self,
        expr: &'b d20::DExpr,
    ) -> LocalBoxFuture<'b, Result<d20::ValTree, DiceRollerError>> {
        async {
            let to_wait = DiceRoller::roll(self, expr).map_err(DiceRollerError::BadExpression)?;
            to_wait
                .await
                .map_err(|err| DiceRollerError::RollerError(err.to_string()))
        }
        .boxed_local()
    }
}
