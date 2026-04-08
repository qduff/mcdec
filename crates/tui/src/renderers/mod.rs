use std::fmt::{self, Display};

use mcd_traits::{display_with_resolver, AddressResolver, TBlock, GenericTerminator, TInstruction, TFunction};

fn chars_needed(n: usize) -> usize {
    if n == 0 {
        1
    } else {
        (n as f64).log10().floor() as usize + 1
    }
}

mod arrow_drawing;

/// TODO use other display with CTX trait and remove this one
pub struct FunctionDisplay<'a, F, B, I, R>
where
    F: TFunction<B, I> + 'a,
    B: TBlock<I> + 'a,
    I: TInstruction + 'a,
    R: AddressResolver  + 'a,
{
    function: &'a F,
    resolver: &'a R,
    _phantom: std::marker::PhantomData<(B, I)>,
}

// TODO update to use ratatui UI elements instead of Display
impl<'a, F, B, I, R> Display for FunctionDisplay<'a, F, B, I, R>
where
    F: TFunction<B, I>,
    B: TBlock<I>,
    I: TInstruction,
    R: AddressResolver,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let function = self.function;
        let mut arrows = arrow_drawing::Arrows::new();

        let blockidx_to_idx = |target| {
            function
                .get_blocks_for_function()
                .take(target)
                .map(|f| f.len())
                .sum()
        };

        function
            .get_blocks_for_function()
            .enumerate()
            .for_each(|(idx, b)| {
                let end = blockidx_to_idx(idx) + b.len() - 1;
                match b.get_block_terminator() {
                    GenericTerminator::Jump { target } => arrows.add_arrow(
                        end,
                        blockidx_to_idx(target),
                        arrow_drawing::ArrowType::Always,
                    ),
                    GenericTerminator::BranchTrue {
                        target_true,
                        target_false,
                    } => {
                        arrows.add_arrow(
                            end,
                            blockidx_to_idx(target_true),
                            arrow_drawing::ArrowType::IfTrue,
                        );
                        arrows.add_arrow(
                            end,
                            blockidx_to_idx(target_false),
                            arrow_drawing::ArrowType::IfFalse,
                        );
                    }
                    GenericTerminator::Return => {}
                }
            });

        for (idx, block) in function.get_blocks_for_function().enumerate() {
            let block_idx = blockidx_to_idx(idx);
            arrows.render_at_addr(f, block_idx, true)?;
            writeln!(f, "{}", block.get_block_name())?;

            let acrv = if let Some((start_fn_addr, end_fn_addr)) = block.get_block_address_bounds()
            {
                Some((
                    start_fn_addr,
                    chars_needed(end_fn_addr),
                ))
            } else {
                None
            };

            for (idx, (addr, instruction)) in block.get_instructions_for_block().enumerate() {
                arrows.render_at_addr(f, block_idx + idx, false)?;

                if let Some((f_addr, f_addr_needed)) = acrv {
                    write!(f, " {:>width$} ", addr.unwrap(), width = f_addr_needed)?;
                }
                write!(f, " {:<3} ", block_idx + idx)?;
                writeln!(f, "{}", display_with_resolver(instruction, self.resolver))?;
            }
        }
        Ok(())
    }
}

pub fn render_function<'a, F, B, I, R>(function: &'a F, resolver: &'a R) -> FunctionDisplay<'a, F, B, I, R>
where
    F: TFunction<B, I>,
    B: TBlock<I>,
    I: TInstruction,
    R: AddressResolver  + 'a,
{
    FunctionDisplay {
        function,
        resolver,
        _phantom: std::marker::PhantomData,
    }
}
