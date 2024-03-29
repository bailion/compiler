use cranelift_codegen::ir::{self, InstBuilder};
use cranelift_module::Module;

use crate::{
    diagnostics::{reportable_error::ReportableError, span::HasSpan},
    parse::{expr::Constructor, table::ParseTable},
    ty::{PrimitiveType, Ty},
};

use super::func::FunctionCompiler;

/// Returns the size (in bytes) of the type in question.
///
/// Note that if the type in question is a pointer, this will return the size of the _pointer_.
pub(crate) fn type_size(ty: Ty) -> u32 {
    match ty {
        // integers are 32 bits (maybe)
        Ty::PrimitiveType(PrimitiveType::Int) => 8,
        // this is quite big, but we can always adjust it later
        // note: all bool need to be converted to/from integers when they are
        // stored/loaded in a stack slot
        Ty::PrimitiveType(PrimitiveType::Bool) => 8,
        _ => todo!(),
    }
}

impl<'ctx, 'builder> FunctionCompiler<'ctx, 'builder> {
    /// Compiles a constructor for a record.
    ///
    /// For example, for the following record (a possible implementation of vector).
    /// ```ignore
    /// record Vector
    ///   start of Int
    ///   capacity of Int
    ///   len of Int
    /// endrecord
    /// ```
    ///
    /// A stack slot would need to be populated as follows.
    ///
    /// ```ignore
    ///   ┌─────────────────────────┐
    ///   │                         │           
    ///   │                         │           
    ///   │                         │
    ///   │                         │           
    ///   │  machine pointer width  │           
    ///   │         integer         │ start     
    ///   │                         │           
    ///   │                         │           
    ///   │                         │           
    ///   │                         │           
    ///   │─────────────────────────┤           
    ///   │                         │           
    ///   │                         │           
    ///   │                         │           
    ///   │                         │           
    ///   │  machine pointer width  │ capacity  
    ///   │         integer         │           
    ///   │                         │           
    ///   │                         │           
    ///   │                         │           
    ///   │                         │           
    ///   │─────────────────────────┤           
    ///   │                         │           
    ///   │                         │           
    ///   │                         │           
    ///   │  machine pointer width  │           
    ///   │         integer         │ len       
    ///   │                         │           
    ///   │                         │           
    ///   │                         │           
    ///   │                         │           
    ///   └─────────────────────────┘           
    ///                                         
    ///          A stack slot
    /// ```
    ///
    /// The stack slot, the slot would be divided into three pieces. If we assume that all the
    /// machine pointer width integers are 32 bits then the stack slot should be 32 * 3 units long.
    ///
    /// Then, if we want to access the value of one of the fields we just compute the size of the
    /// two previous fields, and use this to work out where our pointer should be. We know where
    /// the stack slot starts (or Cranelift does at least) and we know how far we are from that
    /// address to the pointer to (for example) `len` would simply be
    /// `stack_slot_pointer + size(start) + size(capacity)`.
    /// TODO: padding
    pub(crate) fn compile_constructor(
        &mut self,
        con: &Constructor,
        table: &ParseTable,
    ) -> Result<ir::Value, ReportableError> {
        let slot = self.builder.create_sized_stack_slot(ir::StackSlotData::new(
            ir::StackSlotKind::ExplicitSlot,
            {
                // TODO: should report failed inference/could not find record earlier than this
                con.fields
                    .iter()
                    .map(|(_, expr)| -> Result<u32, ReportableError> {
                        let ty = self.ty_env.ty_of(expr.id).ok_or_else(|| {
                            ReportableError::could_not_infer_ty(table.get_expr(expr).span(table))
                        })?;
                        Ok(type_size(ty))
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .sum()
            },
        ));

        let mut offset = 0;
        for expr in con.fields.values() {
            let expr_value = self.compile_expr(table.get_expr_with_id(*expr), table)?;
            let expr_value = if self.builder.func.dfg.value_type(expr_value).is_bool() {
                self.builder.ins().bint(ir::types::I32, expr_value)
            } else {
                expr_value
            };
            self.builder.ins().stack_store(expr_value, slot, offset);
            offset += type_size(self.ty_env.ty_of(expr.id).unwrap()) as i32;
        }

        Ok(self
            .builder
            .ins()
            .stack_addr(self.module.target_config().pointer_type(), slot, 0))
    }
}
