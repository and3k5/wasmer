//! Dummy Engine.

use crate::DummyArtifact;
use std::sync::Arc;
use wasm_common::FunctionType;
use wasmer_compiler::CompileError;
#[cfg(feature = "compiler")]
use wasmer_compiler::{Compiler, CompilerConfig};
use wasmer_engine::{Artifact, DeserializeError, Engine, Tunables};
use wasmer_runtime::{
    SignatureRegistry, VMContext, VMFunctionBody, VMSharedSignatureIndex, VMTrampoline,
};

extern "C" fn dummy_trampoline(
    _context: *mut VMContext,
    _body: *const VMFunctionBody,
    _values: *mut u128,
) {
    panic!("Dummy engine can't call functions, since Wasm function bodies are not really compiled")
}

/// A WebAssembly `Dummy` Engine.
#[derive(Clone)]
pub struct DummyEngine {
    signatures: Arc<SignatureRegistry>,
    tunables: Arc<dyn Tunables + Send + Sync>,
}

impl DummyEngine {
    #[cfg(feature = "compiler")]
    pub fn new(tunables: impl Tunables + 'static + Send + Sync) -> Self {
        Self {
            signatures: Arc::new(SignatureRegistry::new()),
            tunables: Arc::new(tunables),
        }
    }
}

impl Engine for DummyEngine {
    /// Get the tunables
    fn tunables(&self) -> &dyn Tunables {
        &*self.tunables
    }

    /// Register a signature
    fn register_signature(&self, func_type: &FunctionType) -> VMSharedSignatureIndex {
        self.signatures.register(func_type)
    }

    /// Lookup a signature
    fn lookup_signature(&self, sig: VMSharedSignatureIndex) -> Option<FunctionType> {
        self.signatures.lookup(sig)
    }

    /// Retrieves a trampoline given a signature
    fn function_call_trampoline(&self, sig: VMSharedSignatureIndex) -> Option<VMTrampoline> {
        Some(dummy_trampoline)
    }

    /// Validates a WebAssembly module
    fn validate(&self, binary: &[u8]) -> Result<(), CompileError> {
        // We mark all Wasm modules as valid
        Ok(())
    }

    /// Compile a WebAssembly binary
    fn compile(&self, binary: &[u8]) -> Result<Arc<dyn Artifact>, CompileError> {
        Ok(Arc::new(DummyArtifact::new(&self, &binary)?))
    }

    /// Deserializes a WebAssembly module (binary content of a Shared Object file)
    unsafe fn deserialize(&self, bytes: &[u8]) -> Result<Arc<dyn Artifact>, DeserializeError> {
        Ok(Arc::new(DummyArtifact::deserialize(&self, &bytes)?))
    }
}
