//! Dissasembler - finds functions, and breaks them up into basic blocks.

use std::collections::BTreeSet;

use mcd_traits::{GenericTerminator, TBlock, TFunction, TInstruction};
use prgparser::opcodes::Opcode;
use prgparser::{addressed_container::AddressedContainer, constants::LocalAddress};

#[derive(Debug)]
pub struct DisassemblyFunction {
    pub blocks: Vec<BasicBlock>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DisassemblyError {
    CannotDissasembleJSR,
}

fn find_index_in_btreeset<T: Ord>(set: &BTreeSet<T>, item: &T) -> Option<usize> {
    if !set.contains(item) {
        return None;
    }
    Some(set.range(..item).count())
}

impl DisassemblyFunction {
    pub fn disassemble(
        slice: &AddressedContainer<Opcode>,
    ) -> Result<DisassemblyFunction, DisassemblyError> {
        Ok(DisassemblyFunction {
            blocks: DisassemblyFunction::make_blocks(slice)?,
        })
    }

    fn make_blocks(
        slice: &AddressedContainer<Opcode>,
    ) -> Result<Vec<BasicBlock>, DisassemblyError> {
        // Find leaders
        let mut leaders: BTreeSet<usize> = BTreeSet::new();
        leaders.insert(slice.start_addr().unwrap());

        let function_end_addr = slice.end_addr().unwrap();

        for (addr, opcode) in slice.iter() {
            match opcode {
                Opcode::GOTO(target) | Opcode::BT(target) | Opcode::BF(target) => {
                    assert!(target.get_local_address() <= function_end_addr as u32);
                    leaders.insert(target.get_local_address() as usize);

                    let next_addr = slice.addr_offset_by_idx(addr, 1).unwrap();
                    leaders.insert(next_addr);
                }
                Opcode::RETURN => {
                    let next_addr = slice.addr_offset_by_idx(addr, 1);
                    if let Some(next_addr) = next_addr {
                        if next_addr < function_end_addr {
                            leaders.insert(next_addr);
                        }
                    }
                }
                Opcode::JSR(_) => return Err(DisassemblyError::CannotDissasembleJSR),
                _ => {}
            }
        }

        // Given leaders, form Basic Blocks
        let mut blocks = Vec::new();

        let mut iter = leaders.iter().enumerate().peekable();
        while let Some((i, block_start_addr)) = iter.next() {
            let block_start_idx = slice.addr_to_idx(*block_start_addr).unwrap();
            let next_block_idx = match iter.peek() {
                Some(&(_, next_block_start)) => slice.addr_to_idx(*next_block_start),
                None => slice.addr_to_idx(function_end_addr).map(|z| z + 1),
            }
            .unwrap();

            let next_addr = slice.idx_to_addr(next_block_idx);

            let new_view = slice.slice(block_start_idx..next_block_idx).unwrap();

            let terminator: DisassemblyTerminator = match new_view.last().unwrap() {
                Opcode::GOTO(x) => DisassemblyTerminator::Jump {
                    target: find_index_in_btreeset(&leaders, &(x.get_local_address() as usize))
                        .unwrap(),
                },
                Opcode::RETURN => DisassemblyTerminator::Return,
                Opcode::BT(x) => DisassemblyTerminator::BranchTrue {
                    target_true: find_index_in_btreeset(
                        &leaders,
                        &(x.get_local_address() as usize),
                    )
                    .unwrap(),
                    target_false: find_index_in_btreeset(&leaders, &(next_addr.unwrap())).unwrap(),
                },
                Opcode::BF(x) => DisassemblyTerminator::BranchTrue {
                    target_true: find_index_in_btreeset(&leaders, &(next_addr.unwrap())).unwrap(),
                    target_false: find_index_in_btreeset(
                        &leaders,
                        &(x.get_local_address() as usize),
                    )
                    .unwrap(),
                },
                _ => DisassemblyTerminator::Jump {
                    // fallthrough
                    target: find_index_in_btreeset(&leaders, &(next_addr.unwrap())).unwrap(),
                },
            };

            blocks.push(BasicBlock {
                name: format!("b{i}"),
                container: new_view,
                terminator,
            });
        }
        Ok(blocks)
    }
}

#[derive(Debug)]
pub struct BasicBlock {
    pub name: String,
    pub container: AddressedContainer<Opcode>,
    pub terminator: DisassemblyTerminator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisassemblyTerminator {
    Jump {
        target: usize,
    },
    BranchTrue {
        target_true: usize,
        target_false: usize,
    },
    Return,
}

impl TFunction<BasicBlock, Opcode> for DisassemblyFunction {
    fn get_blocks_for_function<'b>(&'b self) -> impl Iterator<Item = &'b BasicBlock>
    where
        BasicBlock: 'b,
    {
        self.blocks.iter()
    }
}

impl TBlock<Opcode> for BasicBlock {
    fn get_block_name(&self) -> &String {
        &self.name
    }

    fn len(&self) -> usize {
        self.container.len()
    }

    fn get_block_terminator(&self) -> GenericTerminator {
        match self.terminator {
            DisassemblyTerminator::Jump { target } => GenericTerminator::Jump { target },
            DisassemblyTerminator::BranchTrue {
                target_true,
                target_false,
            } => GenericTerminator::BranchTrue {
                target_true,
                target_false,
            },
            DisassemblyTerminator::Return => GenericTerminator::Return,
        }
    }

    fn get_block_address_bounds(&self) -> Option<(usize, usize)> {
        self.container.start_addr().zip(self.container.end_addr())
    }

    fn get_instructions_for_block<'b>(&'b self) -> impl Iterator<Item = (Option<usize>, &'b Opcode)>
    where
        Opcode: 'b,
        Opcode: TInstruction,
    {
        self.container.iter().map(|(a, o)| (Some(a), o))
    }
}
