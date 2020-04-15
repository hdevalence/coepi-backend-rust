#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Shard(pub u64);

impl std::str::FromStr for Shard {
    type Err = std::num::ParseIntError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Ok(Self(input.parse()?))
    }
}
