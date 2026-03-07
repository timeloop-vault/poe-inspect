/// A parsed stat description file containing all descriptions and directives.
#[derive(Debug)]
pub struct StatDescriptionFile {
    pub includes: Vec<String>,
    pub no_descriptions: Vec<String>,
    pub descriptions: Vec<StatDescription>,
}

/// A single description block mapping stat IDs to display text.
#[derive(Debug)]
pub struct StatDescription {
    pub stat_ids: Vec<String>,
    pub languages: Vec<LangBlock>,
}

/// Variants for a specific language (or English if no lang specified).
#[derive(Debug)]
pub struct LangBlock {
    pub language: Option<String>,
    pub variants: Vec<Variant>,
}

/// A single variant line: conditions under which this text template applies.
#[derive(Debug)]
pub struct Variant {
    pub ranges: Vec<Range>,
    pub format_string: String,
    pub transforms: Vec<Transform>,
    pub is_canonical: bool,
    pub canonical_stat: Option<usize>,
    pub reminder_id: Option<String>,
}

/// A value range condition for a variant line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Range {
    /// `#` — any value
    Any,
    /// `N` — exactly N
    Exact(i64),
    /// `!N` — not N
    Not(i64),
    /// `N|M` — N to M inclusive (also covers `#|N` and `N|#`)
    Between(Bound, Bound),
}

/// A bound in a range expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bound {
    /// `#` — unbounded
    Unbounded,
    /// A specific integer value
    Value(i64),
}

/// A value transform applied to a stat before display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transform {
    pub kind: TransformKind,
    pub stat_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransformKind {
    Negate,
    NegateAndDouble,
    Double,
    MillisecondsToSeconds,
    MillisecondsToSeconds0dp,
    MillisecondsToSeconds1dp,
    MillisecondsToSeconds2dp,
    MillisecondsToSeconds2dpIfRequired,
    DecisecondsToSeconds,
    PerMinuteToPerSecond,
    PerMinuteToPerSecond0dp,
    PerMinuteToPerSecond1dp,
    PerMinuteToPerSecond2dp,
    PerMinuteToPerSecond2dpIfRequired,
    DivideByTwo0dp,
    DivideByThree,
    DivideByFour,
    DivideByFive,
    DivideBySix,
    DivideByTen0dp,
    DivideByTen1dp,
    DivideByTen1dpIfRequired,
    DivideByTwelve,
    DivideByFifteen0dp,
    DivideByTwenty,
    DivideByTwentyThenDouble0dp,
    DivideByOneHundred,
    DivideByOneHundred2dp,
    DivideByOneHundredAndNegate,
    DivideByOneHundred2dpIfRequired,
    DivideByOneThousand,
    TimesOnePointFive,
    TimesTwenty,
    PlusTwoHundred,
    ThirtyPercentOfValue,
    SixtyPercentOfValue,
    PermyriadPerMinuteToPercentPerSecond,
    OldLeechPercent,
    OldLeechPermyriad,
    MultiplicativeDamageModifier,
    ModValueToItemClass,
    DisplayIndexableSupport,
    DisplayIndexableSkill,
    PassiveHash,
    AfflictionRewardType,
    LocationsToMetres,
    TreeExpansionJewelPassive,
    WeaponTreeUniqueBaseTypeName,
    /// Unknown transform — new ones appear each league
    Other(String),
}

impl Range {
    /// Check whether a value matches this range condition.
    pub fn matches(&self, value: i64) -> bool {
        match self {
            Range::Any => true,
            Range::Exact(n) => value == *n,
            Range::Not(n) => value != *n,
            Range::Between(lo, hi) => {
                let lo_ok = match lo {
                    Bound::Unbounded => true,
                    Bound::Value(n) => value >= *n,
                };
                let hi_ok = match hi {
                    Bound::Unbounded => true,
                    Bound::Value(n) => value <= *n,
                };
                lo_ok && hi_ok
            }
        }
    }
}
