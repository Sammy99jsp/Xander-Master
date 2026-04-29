use dynx::{Identity, IntoNamespace, Namespace};
use rkyv::{Archive, Deserialize, Serialize};
use xander_runtime::ui;

pub struct NS;

impl Namespace for NS {
    const ID: &'static str = "CURRENCY";
}

pub trait Coin: Identity<Parent = Currency> + ui::Ui {
    fn value(&self) -> CP;
    fn amount(&self) -> u32;

    #[doc(hidden)]
    fn currency(currency: &Currency) -> &Self;

    #[doc(hidden)]
    fn currency_mut(currency: &mut Currency) -> &mut Self;
}

macro_rules! coins {
    {
        $(
            $(#[$($tt: tt)*])*
            pub struct $coin: ident @ $val: literal CP @ $id: ident;
        )*
    } => {
        $(
            $(#[$($tt)*])*
            pub struct $coin(pub u32);

            impl Identity for $coin {
                type Parent = Currency;
                const LOCAL_ID: &'static str = stringify!($coin);
            }

            impl Coin for $coin {
                fn value(&self) -> CP {
                    CP(u32::saturating_mul(self.0, $val))
                }

                fn amount(&self) -> u32 {
                    self.0
                }

                fn currency(currency: &Currency) -> &Self {
                    &currency.$id
                }

                fn currency_mut(currency: &mut Currency) -> &mut Self {
                    &mut currency.$id
                }
            }

            impl ui::Ui for $coin {}
        )*
    };
}

/// Collection of all the currency a creature has in its possession.
#[derive(Debug, Clone, Copy, Archive, Serialize, Deserialize)]
pub struct Currency {
    pub cp: CP,
    pub sp: SP,
    pub ep: EP,
    pub gp: GP,
    pub pp: PP,
}

impl IntoNamespace for Currency {
    type Namespace = NS;
}

coins! {
    /// Copper Piece (CP)
    #[derive(Debug, Clone, Copy, Archive, Serialize, Deserialize)]
    pub struct CP @ 1 CP @ cp;

    /// Silver Piece (SP)
    #[derive(Debug, Clone, Copy, Archive, Serialize, Deserialize)]
    pub struct SP @ 10 CP @ sp;

    /// Electrum Piece (EP)
    #[derive(Debug, Clone, Copy, Archive, Serialize, Deserialize)]
    pub struct EP @ 50 CP @ ep;

    /// Gold Piece (GP)
    #[derive(Debug, Clone, Copy, Archive, Serialize, Deserialize)]
    pub struct GP @ 100 CP @ gp;

    /// Platinum Piece (PP)
    #[derive(Debug, Clone, Copy, Archive, Serialize, Deserialize)]
    pub struct PP @ 1000 CP @ pp;
}
