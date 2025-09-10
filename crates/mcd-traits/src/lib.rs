use std::fmt::{self, Debug};

type BlockIdx = usize;

pub enum GenericTerminator {
    Jump {
        target: BlockIdx,
    },
    BranchTrue {
        target_true: BlockIdx,
        target_false: BlockIdx,
    },
    Return,
}

pub trait TFunction<B, I>: Debug {
    fn get_blocks_for_function<'a>(&'a self) -> impl Iterator<Item = &'a B>
    where
        B: 'a,
        B: TBlock<I>;
}

pub trait TBlock<I>: Debug {
    fn get_block_name(&self) -> &String;

    fn get_block_terminator(&self) -> GenericTerminator;

    fn get_block_address_bounds(&self) -> Option<(usize, usize)>;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn get_instructions_for_block<'a>(&'a self) -> impl Iterator<Item = (Option<usize>, &'a I)>
    where
        I: 'a,
        I: TInstruction;
}

pub trait TInstruction: DisplayWithResolver {}

pub trait AddressResolver {
    fn resolve_symbol(&self, addr: u32) -> Option<&str>;
    fn resolve_data(&self, addr: u32) -> Option<&str>;
}

/// Analagous to display but with extra resolver context, allowing looking up symbols
pub trait DisplayWithResolver {
    fn fmt_with_resolver<R: AddressResolver>(
        &self,
        f: &mut fmt::Formatter<'_>,
        resolver: &R,
    ) -> fmt::Result;
}

pub struct DisplayWrapper<'a, T, R> {
    inner: &'a T,
    resolver: &'a R,
}

impl<'a, T, R> fmt::Display for DisplayWrapper<'a, T, R>
where
    T: DisplayWithResolver,
    R: AddressResolver,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt_with_resolver(f, self.resolver)
    }
}

pub fn display_with_resolver<'a, T: DisplayWithResolver, R: AddressResolver>(
    inner: &'a T,
    resolver: &'a R,
) -> DisplayWrapper<'a, T, R> {
    DisplayWrapper { inner, resolver }
}
