//! SSA form

use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    fmt::{self, Debug},
};

use prgparser::{
    constants::{DataAddress, SymbolAddress},
    opcodes::Opcode,
};

use crate::disassembler::DisassemblyFunction;

use mcd_traits::{display_with_resolver, AddressResolver, DisplayWithResolver, TBlock, TFunction, GenericTerminator, TInstruction};


#[derive(Debug)]
pub struct SSAFunction {
    pub blocks: Vec<SSABlock>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Variable {
    Local { addr: u8, unique: u32 },
    Stack { addr: u32, unique: u32 },
    Internal { unique: u32 },
}

impl Debug for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Variable::Local { addr, unique } => f.write_fmt(format_args!("L{addr:?}_{unique:?}")),
            Variable::Stack { addr, unique } => f.write_fmt(format_args!("S{addr:?}_{unique:?}")),
            Variable::Internal { unique } => f.write_fmt(format_args!("I_{unique:?}")),
        }
    }
}

struct FunctionVariableContext {
    stack_unique: HashMap<u32, u32>, 
    local_unique: HashMap<u8, u32>,
    internal_unique: Option<u32>,
}

impl<'a> FunctionVariableContext {
    fn new_ctx() -> Self {
        FunctionVariableContext {
            stack_unique: HashMap::new(),
            local_unique: HashMap::new(),
            internal_unique: None,
        }
    }

    fn new_block_context(&'a mut self, sp: u32) -> BlockVariableContext<'a> {
        BlockVariableContext {
            cur_sp: sp,
            ctx: self,
        }
    }
}

struct BlockVariableContext<'a> {
    ctx: &'a mut FunctionVariableContext,
    cur_sp: u32,
}

impl BlockVariableContext<'_> {
    fn get_sp(&self) -> u32 {
        self.cur_sp
    }
    
    fn set_internal(&mut self) -> Variable {
        if let Some(internal_unique) = self.ctx.internal_unique.as_mut() {
            *internal_unique += 1;
            Variable::Internal {
                unique: *internal_unique,
            }
        } else {
            self.ctx.internal_unique = Some(0);
            Variable::Internal { unique: 0 }
        }
    }

    fn get_internal(&mut self) -> Variable {
        if let Some(unique) = self.ctx.internal_unique {
            Variable::Internal { unique }
        } else {
            panic!("Malformed program: Internal variable not set!!!")
        }
    }

    fn new_local_at_idx(&mut self, idx: u8) -> Variable {
        let unique = *self
            .ctx
            .local_unique
            .entry(idx)
            .and_modify(|unique| *unique += 1)
            .or_insert(0);

        Variable::Local { addr: idx, unique }
    }
    
    fn get_local(&mut self, idx: u8) -> Variable {
        let unique = *self.ctx.local_unique.entry(idx).or_insert(0);
        Variable::Local { addr: idx, unique }
    }

    fn push_stack_variable(&mut self) -> Variable {
        self.cur_sp += 1;

        let unique = *self
            .ctx
            .stack_unique
            .entry(self.cur_sp)
            .and_modify(|unique| *unique += 1)
            .or_insert(0);

        Variable::Stack {
            addr: self.cur_sp,
            unique,
        }
    }

    fn get_stack_variable_top_offset(&mut self, offset: u32) -> Variable {
        let addr = self.cur_sp.checked_sub(offset).expect("indexing too far!");
        let unique = *self.ctx.stack_unique.entry(addr).or_insert(0);
        Variable::Stack { addr, unique }
    }

    fn get_stack_variable_top(&mut self) -> Variable {
        self.get_stack_variable_top_offset(0)
    }

    fn pop_stack_variable_top(&mut self) -> Variable {
        let ret = self.get_stack_variable_top();
        self.cur_sp = self.cur_sp.checked_sub(1).expect("popped too far!!");
        ret
    }
}

#[derive(Debug)]
enum ImmediateValue {
    Null,
    Integer(i32),
    Long(u64),
    Float(f32),
    Double(f64),
    Char(char),
    String(DataAddress),
    Symbol(SymbolAddress),
    Boolean(bool),
}

impl Debug for OP {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OP::Move { src, dest } => f.write_fmt(format_args!("{:?} = {:?}", dest, src)),
            OP::MoveImm { src, dest } => f.write_fmt(format_args!("{:?} = {:?}", dest, src)),
            OP::BinaryOperation { op, src1, src2, dest } => f.write_fmt(format_args!("{dest:?} = {src1:?} {op:?} {src2:?}")),
            OP::UnaryOperation { op, src, dest } => f.write_fmt(format_args!("{dest:?} = {op:?} {src:?}")),
            OP::Getv { base, symbol, dest } => f.write_fmt(format_args!("{dest:?} = {base:?}.{symbol:?}")),
            OP::Putv { src, base, symbol } => f.write_fmt(format_args!("{base:?}.{symbol:?} = {src:?}")),
            OP::Getm { symbol, dest } => f.write_fmt(format_args!("{dest:?} = getm({symbol:?})")),
            OP::Call { function, arguments, dest } => {
                f.write_fmt(format_args!("{dest:?} = {function:?}("))?;
                for arg in arguments {
                    f.write_fmt(format_args!("{arg:?}, "))?;
                }
                f.write_str(")")
            }
            OP::Aputv { src, idx, value } => f.write_fmt(format_args!("{src:?}[{idx:?}] =  {value:?}")),
            OP::Agetv { src, idx, dest } => f.write_fmt(format_args!("{dest:?} = {src:?}[{idx:?}]")),
        }
    }
}

enum UnaryOperator {
    IsNull,
    IsNotNull,
    Inv,
    NewDictionary,
    NewClass,
    NewArray,
    NewByteArray,
}

impl Debug for UnaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnaryOperator::IsNull => f.write_str("isnull"),
            UnaryOperator::IsNotNull => f.write_str("isnotnull"),
            UnaryOperator::Inv => f.write_str("~"),
            UnaryOperator::NewDictionary => f.write_str("newdict"),
            UnaryOperator::NewArray => f.write_str("newarray"),
            UnaryOperator::NewByteArray => f.write_str("newarray"),
            UnaryOperator::NewClass => f.write_str("newclass"),
        }
    }
}

enum BinaryOperator {
    Add, Sub, Mul, Div, And, Or, Mod, Shl, Shr, Xor,
    Eq, Lt, Lte, Gt, Gte, Ne,
    Isa, Has,
}

impl Debug for BinaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryOperator::Add => f.write_str("+"),
            BinaryOperator::Sub => f.write_str("-"),
            BinaryOperator::Mul => f.write_str("*"),
            BinaryOperator::Div => f.write_str("/"),
            BinaryOperator::And => f.write_str("&&"),
            BinaryOperator::Or => f.write_str("||"),
            BinaryOperator::Mod => f.write_str("%"),
            BinaryOperator::Shl => f.write_str("<<"),
            BinaryOperator::Shr => f.write_str(">>"),
            BinaryOperator::Xor => f.write_str("^"),
            BinaryOperator::Eq => f.write_str("=="),
            BinaryOperator::Lt => f.write_str("<"),
            BinaryOperator::Lte => f.write_str("<="),
            BinaryOperator::Gt => f.write_str(">"),
            BinaryOperator::Gte => f.write_str(">="),
            BinaryOperator::Ne => f.write_str("!="),
            BinaryOperator::Isa => f.write_str("isa"),
            BinaryOperator::Has => f.write_str("has"),
        }
    }
}

pub enum OP {
    Move { src: Variable, dest: Variable },
    MoveImm { src: ImmediateValue, dest: Variable },
    BinaryOperation { op: BinaryOperator, src1: Variable, src2: Variable, dest: Variable },
    UnaryOperation { op: UnaryOperator, src: Variable, dest: Variable },
    Getv { base: Variable, symbol: Variable, dest: Variable },
    Getm { symbol: Variable, dest: Variable },
    Aputv { src: Variable, idx: Variable, value: Variable },
    Agetv { src: Variable, idx: Variable, dest: Variable },
    Putv { src: Variable, base: Variable, symbol: Variable },
    Call { function: Variable, arguments: Vec<Variable>, dest: Variable },
}

impl OP {
    fn from_binaryop(op: BinaryOperator, ctx: &mut BlockVariableContext) -> Self {
        OP::BinaryOperation {
            op,
            src2: ctx.pop_stack_variable_top(),
            src1: ctx.pop_stack_variable_top(),
            dest: ctx.push_stack_variable(),
        }
    }

    fn from_unaryop(op: UnaryOperator, ctx: &mut BlockVariableContext) -> Self {
        OP::UnaryOperation {
            op,
            src: ctx.pop_stack_variable_top(),
            dest: ctx.push_stack_variable(),
        }
    }

    fn lgetv_primitive(ctx: &mut BlockVariableContext<'_>, idx: u8) -> [Self; 1] {
        [OP::Move {
            src: ctx.get_local(idx),
            dest: ctx.push_stack_variable(),
        }]
    }

    fn spush_primitive(ctx: &mut BlockVariableContext<'_>, symbol: SymbolAddress) -> [Self; 1] {
        [OP::MoveImm {
            src: ImmediateValue::Symbol(symbol),
            dest: ctx.push_stack_variable(),
        }]
    }

    fn getv_primitive(ctx: &mut BlockVariableContext<'_>) -> [Self; 2] {
        [
            OP::Move {
                src: ctx.get_stack_variable_top_offset(1),
                dest: ctx.set_internal(),
            },
            OP::Getv {
                symbol: ctx.pop_stack_variable_top(),
                base: ctx.pop_stack_variable_top(),
                dest: ctx.push_stack_variable(),
            },
        ]
    }

    fn invokem_primitive(ctx: &mut BlockVariableContext<'_>, count: u8) -> [Self; 1] {
        let mut args = (0..count)
            .map(|_| ctx.pop_stack_variable_top())
            .collect::<Vec<_>>();
        args.reverse();
        [OP::Call {
            arguments: args,
            function: ctx.pop_stack_variable_top(),
            dest: ctx.push_stack_variable(),
        }]
    }

    fn getm_primitive(ctx: &mut BlockVariableContext<'_>) -> [Self; 1] {
        [
            OP::Getm {
                symbol: ctx.pop_stack_variable_top(),
                dest: ctx.push_stack_variable(),
            },
        ]
    }

    fn aputv_primitive(ctx: &mut BlockVariableContext<'_>) -> [Self; 1] {
        [OP::Aputv {
            value: ctx.pop_stack_variable_top(),
            idx: ctx.pop_stack_variable_top(),
            src: ctx.pop_stack_variable_top(),
        }]
    }

    fn dup_primitive(ctx: &mut BlockVariableContext<'_>, offset: u8) -> [Self; 1] {
        [OP::Move {
            src: ctx.get_stack_variable_top_offset(offset as u32),
            dest: ctx.push_stack_variable(),
        }]
    }
}

#[derive(Debug)]
pub struct SSABlock {
     name: String,
     start_depth: u32,
     ops: Vec<OP>,
     terminator: SSATerminator,
}

 enum SSATerminator {
    Jump { target: usize },
    BranchTrue {
        test: Variable,
        target_true: usize,
        target_false: usize,
    },
    Return { var: Variable },
}

impl Debug for SSATerminator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SSATerminator::Jump { target } => f.write_fmt(format_args!("JUMP {target}")),
            SSATerminator::BranchTrue { test, target_true, target_false } => 
                f.write_fmt(format_args!("JUMP {test:?} ? {target_true} : {target_false}")),
            SSATerminator::Return { var } => f.write_fmt(format_args!("RETURN {var:?}")),
        }
    }
}

fn get_operation(opcode: &Opcode, ctx: &mut BlockVariableContext, ops: &mut Vec<OP>) {
    match opcode {
        Opcode::LGETV(idx) => ops.extend(OP::lgetv_primitive(ctx, *idx)),
        Opcode::GETSELF => ops.push(OP::Move {
            src: ctx.get_local(0),
            dest: ctx.push_stack_variable(),
        }),
        Opcode::LPUTV(idx) => ops.push(OP::Move {
            src: ctx.pop_stack_variable_top(),
            dest: ctx.new_local_at_idx(*idx),
        }),
        Opcode::SPUSH(symbol) => ops.extend(OP::spush_primitive(ctx, *symbol)),
        Opcode::NPUSH => ops.push(OP::MoveImm {
            src: ImmediateValue::Null,
            dest: ctx.push_stack_variable(),
        }),
        Opcode::BPUSH(x) => ops.push(OP::MoveImm {
            src: ImmediateValue::Boolean(*x == 1),
            dest: ctx.push_stack_variable(),
        }),

        Opcode::NEWS(string) => ops.push(OP::MoveImm {
            src: ImmediateValue::String(*string),
            dest: ctx.push_stack_variable(),
        }),
        Opcode::IPUSH(imm) => ops.push(OP::MoveImm {
            src: ImmediateValue::Integer(*imm),
            dest: ctx.push_stack_variable(),
        }),
        Opcode::IPUSH1(imm) => ops.push(OP::MoveImm {
            src: ImmediateValue::Integer(*imm as i32),
            dest: ctx.push_stack_variable(),
        }),
        Opcode::IPUSH2(imm) => ops.push(OP::MoveImm {
            src: ImmediateValue::Integer(*imm as i32),
            dest: ctx.push_stack_variable(),
        }),
        Opcode::IPUSH3(imm) => ops.push(OP::MoveImm {
            src: ImmediateValue::Integer(*imm),
            dest: ctx.push_stack_variable(),
        }),
        Opcode::IPUSHZ => ops.push(OP::MoveImm {
            src: ImmediateValue::Integer(0),
            dest: ctx.push_stack_variable(),
        }),
        Opcode::DPUSH(imm) => ops.push(OP::MoveImm {
            src: ImmediateValue::Double(*imm),
            dest: ctx.push_stack_variable(),
        }),
        Opcode::DPUSHZ => ops.push(OP::MoveImm {
            src: ImmediateValue::Double(0f64),
            dest: ctx.push_stack_variable(),
        }),
        Opcode::LPUSH(imm) => ops.push(OP::MoveImm {
            src: ImmediateValue::Long(*imm),
            dest: ctx.push_stack_variable(),
        }),
        Opcode::LPUSHZ => ops.push(OP::MoveImm {
            src: ImmediateValue::Long(0),
            dest: ctx.push_stack_variable(),
        }),
        Opcode::CPUSH(imm) => ops.push(OP::MoveImm {
            src: ImmediateValue::Char(*imm),
            dest: ctx.push_stack_variable(),
        }),

        Opcode::FPUSH(imm) => ops.push(OP::MoveImm {
            src: ImmediateValue::Float(*imm),
            dest: ctx.push_stack_variable(),
        }),
        Opcode::FPUSHZ => ops.push(OP::MoveImm {
            src: ImmediateValue::Float(0f32),
            dest: ctx.push_stack_variable(),
        }),
        Opcode::BTPUSH => ops.push(OP::MoveImm {
            src: ImmediateValue::Boolean(true),
            dest: ctx.push_stack_variable(),
        }),
        Opcode::BFPUSH => ops.push(OP::MoveImm {
            src: ImmediateValue::Boolean(false),
            dest: ctx.push_stack_variable(),
        }),

        Opcode::ADD => ops.push(OP::from_binaryop(BinaryOperator::Add, ctx)),
        Opcode::SUB => ops.push(OP::from_binaryop(BinaryOperator::Sub, ctx)),
        Opcode::MUL => ops.push(OP::from_binaryop(BinaryOperator::Mul, ctx)),
        Opcode::DIV => ops.push(OP::from_binaryop(BinaryOperator::Div, ctx)),
        Opcode::AND => ops.push(OP::from_binaryop(BinaryOperator::And, ctx)),
        Opcode::OR => ops.push(OP::from_binaryop(BinaryOperator::Or, ctx)),
        Opcode::MOD => ops.push(OP::from_binaryop(BinaryOperator::Mod, ctx)),
        Opcode::SHL => ops.push(OP::from_binaryop(BinaryOperator::Shl, ctx)),
        Opcode::SHR => ops.push(OP::from_binaryop(BinaryOperator::Shr, ctx)),
        Opcode::XOR => ops.push(OP::from_binaryop(BinaryOperator::Xor, ctx)),

        Opcode::EQ => ops.push(OP::from_binaryop(BinaryOperator::Eq, ctx)),
        Opcode::LT => ops.push(OP::from_binaryop(BinaryOperator::Lt, ctx)),
        Opcode::LTE => ops.push(OP::from_binaryop(BinaryOperator::Lte, ctx)),
        Opcode::GT => ops.push(OP::from_binaryop(BinaryOperator::Gt, ctx)),
        Opcode::GTE => ops.push(OP::from_binaryop(BinaryOperator::Gte, ctx)),
        Opcode::NE => ops.push(OP::from_binaryop(BinaryOperator::Ne, ctx)),

        Opcode::ISA => ops.push(OP::from_binaryop(BinaryOperator::Isa, ctx)),
        Opcode::CANHAZPLZ => ops.push(OP::from_binaryop(BinaryOperator::Has, ctx)),

        Opcode::INV => ops.push(OP::from_unaryop(UnaryOperator::Inv, ctx)),
        Opcode::ISNULL => ops.push(OP::from_unaryop(UnaryOperator::IsNull, ctx)),
        Opcode::ISNOTNULL => ops.push(OP::from_unaryop(UnaryOperator::IsNotNull, ctx)),

        Opcode::NEWD => ops.push(OP::from_unaryop(UnaryOperator::NewDictionary, ctx)),
        Opcode::NEWA => ops.push(OP::from_unaryop(UnaryOperator::NewArray, ctx)),
        Opcode::NEWBA => ops.push(OP::from_unaryop(UnaryOperator::NewByteArray, ctx)),
        Opcode::NEWC => ops.push(OP::from_unaryop(UnaryOperator::NewClass, ctx)),

        Opcode::AGETV => ops.push(OP::Agetv {
            idx: ctx.pop_stack_variable_top(),
            src: ctx.pop_stack_variable_top(),
            dest: ctx.push_stack_variable(),
        }),

        Opcode::APUTV => ops.extend(OP::aputv_primitive(ctx)),

        Opcode::DUP(offset) => ops.extend(OP::dup_primitive(ctx, *offset)),

        Opcode::GETV => {
            ops.extend(OP::getv_primitive(ctx));
        }

        Opcode::PUTV => ops.push(OP::Putv {
            src: ctx.pop_stack_variable_top(),
            symbol: ctx.pop_stack_variable_top(),
            base: ctx.pop_stack_variable_top(),
        }),
        Opcode::FRPUSH => ops.push(OP::Move {
            src: ctx.get_internal(),
            dest: ctx.push_stack_variable(),
        }),
        Opcode::INVOKEM(count) => {
            ops.extend(OP::invokem_primitive(ctx, *count));
        }
        Opcode::INVOKEMZ => {
            ops.extend(OP::lgetv_primitive(ctx, 0));
            ops.extend(OP::invokem_primitive(ctx, 1));
        }

        Opcode::GETM => {
            ops.extend(OP::getm_primitive(ctx));
        }
        Opcode::POPV => {
            ctx.pop_stack_variable_top();
        }

        Opcode::GETSELFV(symbol) => {
            ops.extend(OP::lgetv_primitive(ctx, 0));
            ops.extend(OP::spush_primitive(ctx, *symbol));
            ops.extend(OP::getv_primitive(ctx));
        }

        Opcode::GETMV(symbol1, symbol2) => {
            ops.extend(OP::spush_primitive(ctx, *symbol1));
            ops.extend(OP::getm_primitive(ctx));
            ops.extend(OP::spush_primitive(ctx, *symbol2));
            ops.extend(OP::getv_primitive(ctx));
        }
        Opcode::GETSV(symbol) => {
            ops.extend(OP::spush_primitive(ctx, *symbol));
            ops.extend(OP::getv_primitive(ctx));
        }

        Opcode::GETLOCALV(idx, symbol) => {
            ops.extend(OP::lgetv_primitive(ctx, *idx));
            ops.extend(OP::spush_primitive(ctx, *symbol));
            ops.extend(OP::getv_primitive(ctx));
        }
        Opcode::APUTVDUP => {
            ops.extend(OP::aputv_primitive(ctx));
            ops.extend(OP::dup_primitive(ctx, 1));
        }

        Opcode::ARGC(_) | Opcode::INCSP(_) | Opcode::ARGCINCSP(_, _) => {}
        Opcode::BT(_) | Opcode::BF(_) | Opcode::RETURN | Opcode::GOTO(_) => {}
        _ => todo!("{:?}", opcode),
    };
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SSAError {
    UnbalancedStack,
}

pub fn perform_ssa_function(
    disassembly_function: &DisassemblyFunction,
) -> Result<SSAFunction, SSAError> {
    let mut ssa_blocks: BTreeMap<usize, SSABlock> = BTreeMap::new();
    let mut search_deque: VecDeque<(usize, u32)> = VecDeque::new();
    search_deque.push_back((0, 0));
    let mut function_ctx = FunctionVariableContext::new_ctx();

    while let Some((disas_block_idx, start_depth)) = search_deque.pop_front() {
        if let Some(blockid) = ssa_blocks.get(&disas_block_idx) {
            if blockid.start_depth != start_depth {
                return Err(SSAError::UnbalancedStack);
            }
            continue;
        }

        let disas_block = &disassembly_function.blocks[disas_block_idx];
        let mut block_ctx = function_ctx.new_block_context(start_depth);
        let mut ops = Vec::new();

        if disas_block_idx == 0 {
            ops.push(OP::MoveImm {
                src: ImmediateValue::Null,
                dest: block_ctx.push_stack_variable(),
            });
        }

        for (_, opc) in disas_block.container.iter() {
            get_operation(opc, &mut block_ctx, &mut ops);
        }

        let terminator = match disas_block.terminator {
            crate::disassembler::DisassemblyTerminator::Jump { target } => {
                search_deque.push_back((target, block_ctx.get_sp()));
                SSATerminator::Jump { target }
            }
            crate::disassembler::DisassemblyTerminator::BranchTrue {
                target_true,
                target_false,
            } => {
                let ret = SSATerminator::BranchTrue {
                    test: block_ctx.pop_stack_variable_top(),
                    target_true,
                    target_false,
                };
                search_deque.push_back((target_true, block_ctx.get_sp()));
                search_deque.push_back((target_false, block_ctx.get_sp()));
                ret
            }
            crate::disassembler::DisassemblyTerminator::Return => SSATerminator::Return {
                var: block_ctx.pop_stack_variable_top(),
            },
        };

        let block = SSABlock {
            name: format!("block_{disas_block_idx}"),
            start_depth,
            ops,
            terminator,
        };
        ssa_blocks.insert(disas_block_idx, block);
    }

    // due to dead blocks being skipped, the block indices have shifted but the terminators have remained the same.
    // we therefore need to work out the mappings and adjust these
    let mut mapper: HashMap<usize, usize> = HashMap::new();
    for (new_idx, &old_idx) in ssa_blocks.keys().enumerate() {
        mapper.insert(old_idx, new_idx);
    }

    // ... and remap
    let mut mapped_blocks = Vec::with_capacity(ssa_blocks.len());
    for (_, mut block) in ssa_blocks.into_iter() {
        match &mut block.terminator {
            SSATerminator::Jump { target } => {
                *target = *mapper
                    .get(target)
                    .expect("Jump target block unexpectedly missing from mapped SSA blocks!");
            }
            SSATerminator::BranchTrue {
                target_true,
                target_false,
                ..
            } => {
                *target_true = *mapper
                    .get(target_true)
                    .expect("BranchTrue target_true unexpectedly missing!");
                *target_false = *mapper
                    .get(target_false)
                    .expect("BranchTrue target_false unexpectedly missing!");
            }
            SSATerminator::Return { .. } => {}
        }
        mapped_blocks.push(block);
    }

    Ok(SSAFunction {
        blocks: mapped_blocks,
    })
}

impl TInstruction for OP {}

impl TBlock<OP> for SSABlock {
    fn get_block_name(&self) -> &String {
        &self.name
    }

    fn len(&self) -> usize {
        self.ops.len()
    }

    fn get_block_terminator(&self) -> GenericTerminator {
        match &self.terminator {
            SSATerminator::Jump { target } => GenericTerminator::Jump { target: *target },
            SSATerminator::BranchTrue {
                target_true,
                target_false,
                ..
            } => GenericTerminator::BranchTrue {
                target_true: *target_true,
                target_false: *target_false,
            },
            SSATerminator::Return { .. } => GenericTerminator::Return,
        }
    }

    fn get_block_address_bounds(&self) -> Option<(usize, usize)> {
        None
    }

    fn get_instructions_for_block<'a>(&'a self) -> impl Iterator<Item = (Option<usize>, &'a OP)>
    where
        OP: 'a,
        OP: TInstruction,
    {
        self.ops.iter().map(|op| (None, op))
    }
}

impl TFunction<SSABlock, OP> for SSAFunction {
    fn get_blocks_for_function<'a>(&'a self) -> impl Iterator<Item = &'a SSABlock>
    where
        SSABlock: 'a,
        SSABlock: TBlock<OP>,
    {
        self.blocks.iter()
    }
}

impl DisplayWithResolver for OP {
    fn fmt_with_resolver<R: AddressResolver>(
        &self,
        f: &mut fmt::Formatter<'_>,
        _resolver: &R,
    ) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
