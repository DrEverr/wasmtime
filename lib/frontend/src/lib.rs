//! Cranelift IR builder library.
//!
//! Provides a straightforward way to create a Cranelift IR function and fill it with instructions
//! corresponding to your source program written in another language.
//!
//! To get started, create an [`FunctionBuilderContext`](struct.FunctionBuilderContext.html) and
//! pass it as an argument to a [`FunctionBuilder`](struct.FunctionBuilder.html).
//!
//! # Source Language Variables and Cranelift IR values
//!
//! The most interesting feature of this API is that it provides a single way to deal with all your
//! variable problems. Indeed, the [`FunctionBuilder`](struct.FunctionBuilder.html) struct has a
//! type parameter `Variable` that should be instantiated with the type of your Source Language
//! Variables (SLV). Then, through calling the functions
//! [`declare_var`](struct.FunctionBuilder.html#method.declare_var),
//! [`def_var`](struct.FunctionBuilder.html#method.def_var) and
//! [`use_var`](struct.FunctionBuilder.html#method.use_var), the
//! [`FunctionBuilder`](struct.FunctionBuilder.html) will create for you all the Cranelift IR
//! values corresponding to your SLVs.
//!
//! If a SLV is immutable (defined only once), then it will get mapped to one and only Cranelift IR
//! value. If a SLV is mutable ([`def_var`](struct.FunctionBuilder.html#method.def_var) multiple
//! times), then internally a [`SSA`](https://en.wikipedia.org/wiki/Static_single_assignment_form)
//! construction algorithm will automatically create multiple Cranelift IR values that will be
//! returned to you by [`use_var`](struct.FunctionBuilder.html#method.use_var) depending on where
//! you want to use your SLV.
//!
//! The morality is that you should use these three functions to handle all your SLVs, even those
//! that are not present in the source code but artefacts of the translation. For instance, if your
//! source language is expression-based, then you will need to introduce artifical SLVs to store
//! intermediate results of the computation of your expressions. Hence The `Variable` type that you
//! would pass to [`FunctionBuilder`](struct.FunctionBuilder.html) could look like this
//!
//! ```
//! enum Variable {
//!     OriginalSourceVariable(String),
//!     IntermediateExpressionVariable(u32)
//! }
//! ```
//!
//! # Example
//!
//! Here is a pseudo-program we want to transform into Cranelift IR:
//!
//! ```clif
//! function(x) {
//! x, y, z : i32
//! block0:
//!    y = 2;
//!    z = x + y;
//!    jump block1
//! block1:
//!    z = z + y;
//!    brnz y, block2;
//!    z = z - x;
//!    return y
//! block2:
//!    y = y - x
//!    jump block1
//! }
//! ```
//!
//! Here is how you build the corresponding Cranelift IR function using `FunctionBuilderContext`:
//!
//! ```rust
//! extern crate cranelift_codegen;
//! extern crate cranelift_frontend;
//!
//! use cranelift_codegen::entity::EntityRef;
//! use cranelift_codegen::ir::{ExternalName, Function, Signature, AbiParam, InstBuilder};
//! use cranelift_codegen::ir::types::*;
//! use cranelift_codegen::settings::{self, CallConv};
//! use cranelift_frontend::{FunctionBuilderContext, FunctionBuilder, Variable};
//! use cranelift_codegen::verifier::verify_function;
//!
//! fn main() {
//!     let mut sig = Signature::new(CallConv::SystemV);
//!     sig.returns.push(AbiParam::new(I32));
//!     sig.params.push(AbiParam::new(I32));
//!     let mut fn_builder_ctx = FunctionBuilderContext::<Variable>::new();
//!     let mut func = Function::with_name_signature(ExternalName::user(0, 0), sig);
//!     {
//!         let mut builder = FunctionBuilder::<Variable>::new(&mut func, &mut fn_builder_ctx);
//!
//!         let block0 = builder.create_ebb();
//!         let block1 = builder.create_ebb();
//!         let block2 = builder.create_ebb();
//!         let x = Variable::new(0);
//!         let y = Variable::new(1);
//!         let z = Variable::new(2);
//!         builder.declare_var(x, I32);
//!         builder.declare_var(y, I32);
//!         builder.declare_var(z, I32);
//!         builder.append_ebb_params_for_function_params(block0);
//!
//!         builder.switch_to_block(block0);
//!         builder.seal_block(block0);
//!         {
//!             let tmp = builder.ebb_params(block0)[0]; // the first function parameter
//!             builder.def_var(x, tmp);
//!         }
//!         {
//!             let tmp = builder.ins().iconst(I32, 2);
//!             builder.def_var(y, tmp);
//!         }
//!         {
//!             let arg1 = builder.use_var(x);
//!             let arg2 = builder.use_var(y);
//!             let tmp = builder.ins().iadd(arg1, arg2);
//!             builder.def_var(z, tmp);
//!         }
//!         builder.ins().jump(block1, &[]);
//!
//!         builder.switch_to_block(block1);
//!         {
//!             let arg1 = builder.use_var(y);
//!             let arg2 = builder.use_var(z);
//!             let tmp = builder.ins().iadd(arg1, arg2);
//!             builder.def_var(z, tmp);
//!         }
//!         {
//!             let arg = builder.use_var(y);
//!             builder.ins().brnz(arg, block2, &[]);
//!         }
//!         {
//!             let arg1 = builder.use_var(z);
//!             let arg2 = builder.use_var(x);
//!             let tmp = builder.ins().isub(arg1, arg2);
//!             builder.def_var(z, tmp);
//!         }
//!         {
//!             let arg = builder.use_var(y);
//!             builder.ins().return_(&[arg]);
//!         }
//!
//!         builder.switch_to_block(block2);
//!         builder.seal_block(block2);
//!
//!         {
//!             let arg1 = builder.use_var(y);
//!             let arg2 = builder.use_var(x);
//!             let tmp = builder.ins().isub(arg1, arg2);
//!             builder.def_var(y, tmp);
//!         }
//!         builder.ins().jump(block1, &[]);
//!         builder.seal_block(block1);
//!
//!         builder.finalize();
//!     }
//!
//!     let flags = settings::Flags::new(settings::builder());
//!     let res = verify_function(&func, &flags);
//!     println!("{}", func.display(None));
//!     if let Err(errors) = res {
//!         panic!("{}", errors);
//!     }
//! }
//! ```

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "std", deny(unstable_features))]
#![cfg_attr(feature = "cargo-clippy", allow(new_without_default))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        float_arithmetic, mut_mut, nonminimal_bool, option_map_unwrap_or, option_map_unwrap_or_else,
        print_stdout, unicode_not_nfc, use_self
    )
)]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc))]

extern crate cranelift_codegen;

pub use frontend::{FunctionBuilder, FunctionBuilderContext};
pub use variable::Variable;

mod frontend;
mod ssa;
mod variable;

#[cfg(not(feature = "std"))]
mod std {
    extern crate alloc;

    pub use self::alloc::vec;
    pub use core::*;
}