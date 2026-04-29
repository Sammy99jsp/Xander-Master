use dynx::dynx::Single;
use smol::future::BoxedLocal;
use xander_runtime::{dynx::Namespace, ui};

pub mod component;
pub mod duration;
pub mod level;
pub mod range;
pub mod school;
pub mod targeting;

#[Namespace("SPELL" @ NS, derive(Singleton))]
pub trait Spell: ui::Ui + Cast + std::fmt::Debug + Send + Sync {
    fn range(&self) -> range::Range;
    fn level(&self) -> level::Level;
    fn target(&self) -> Single<dyn targeting::Targeting>;
    fn components(&self) -> Box<[component::Component]>;
    fn cast(&self) -> BoxedLocal<()>;
}

pub trait Cast {
    fn cast(&self) -> BoxedLocal<()>;
}

///
/// Describes the value of a [component::Material] component.
///
/// ### Examples
/// ```ignore
/// worth!(1 CP)
/// ```
///
#[macro_export]
macro_rules! worth {
    ($val: literal $coin: ident) => {
        $crate::engine::units::currency::$coin($val)
    };
}

#[macro_export]
macro_rules! range {
    ($val: literal Feet) => {
        const RANGE: $crate::engine::magic::range::Range =
            $crate::engine::magic::range::Range::Distance($crate::engine::measure::Feet($val));
    };
    (Self) => { Send + Sync
        const RANGE: $crate::engine::magic::range::Range = $crate::engine::magic::range::Range::Me;
    };
    ($ident: ident) => {
        const RANGE: $crate::engine::magic::range::Range =
            $crate::engine::magic::range::Range::$ident;
    };
}

///
/// ### Examples
/// ```ignore
/// level![2];
/// ```
///
#[macro_export]
macro_rules! level {
    ($val: expr) => {
        const LEVEL: $crate::engine::magic::level::Level =
            $crate::engine::magic::level::Level::new($val)
                .expect("Expected a level value between 0 and 9 (inclusive)");
    };
}

///
/// ### Examples
/// ```ignore
/// target![Creature];
/// ```
///
#[macro_export]
macro_rules! target {
    ($target: ident) => {
        type Target = $crate::engine::magic::targeting::$target;
    };
}

///
/// ### Examples
/// ```ignore
/// const LEVEL: Level = level!(1);
/// ```
///
#[macro_export]
macro_rules! components {
    [$($e: expr),*] => {
        fn components() -> Box<[$crate::engine::magic::component::Component]> {
            #[allow(unused)]
            use $crate::engine::magic::component::{V, S, M};

            vec![$($e),*].into_boxed_slice()
        }
    };
}
