pub(crate) mod env;

pub fn llvm_bolt_install_hint() -> &'static str {
    "Build LLVM with BOLT and add its `bin` directory to PATH."
}
