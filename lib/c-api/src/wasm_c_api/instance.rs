use super::externals::{wasm_extern_t, wasm_extern_vec_t};
use super::module::wasm_module_t;
use super::store::wasm_store_t;
use super::trap::wasm_trap_t;
use crate::ordered_resolver::OrderedResolver;
use std::mem;
use std::sync::Arc;
use wasmer::{Extern, Instance, InstantiationError};

/// Opaque type representing a WebAssembly instance.
#[allow(non_camel_case_types)]
pub struct wasm_instance_t {
    pub(crate) inner: Arc<Instance>,
}

/// Creates a new instance from a WebAssembly module and a
/// set of imports.
///
/// ## Errors
///
/// The function can fail in 2 ways:
///
/// 1. Link errors that happen when plugging the imports into the
///    instance,
/// 2. Runtime errors that happen when running the module `start`
///    function.
///
/// Failures are stored in the `traps` argument; the program doesn't
/// panic.
///
/// # Notes
///
/// The `store` argument is ignored. The store from the given module
/// will be used.
///
/// # Example
///
/// See the module's documentation.
#[no_mangle]
pub unsafe extern "C" fn wasm_instance_new(
    _store: Option<&wasm_store_t>,
    module: Option<&wasm_module_t>,
    imports: Option<&wasm_extern_vec_t>,
    traps: *mut *mut wasm_trap_t,
) -> Option<Box<wasm_instance_t>> {
    let module = module?;
    let imports = imports?;

    let wasm_module = &module.inner;
    let module_imports = wasm_module.imports();
    let module_import_count = module_imports.len();
    let resolver: OrderedResolver = imports
        .into_slice()
        .map(|imports| imports.iter())
        .unwrap_or_else(|| [].iter())
        .map(|imp| Extern::from((&**imp).clone()))
        .take(module_import_count)
        .collect();

    let instance = match Instance::new(wasm_module, &resolver) {
        Ok(instance) => Arc::new(instance),

        Err(InstantiationError::Link(link_error)) => {
            crate::error::update_last_error(link_error);

            return None;
        }

        Err(InstantiationError::Start(runtime_error)) => {
            let trap: Box<wasm_trap_t> = Box::new(runtime_error.into());
            *traps = Box::into_raw(trap);

            return None;
        }

        Err(InstantiationError::HostEnvInitialization(error)) => {
            crate::error::update_last_error(error);

            return None;
        }
    };

    Some(Box::new(wasm_instance_t { inner: instance }))
}

/// Deletes an instance.
///
/// # Example
///
/// See `wasm_instance_new`.
#[no_mangle]
pub unsafe extern "C" fn wasm_instance_delete(_instance: Option<Box<wasm_instance_t>>) {}

/// Gets the exports of the instance.
///
/// # Example
///
/// ```rust
/// # use inline_c::assert_c;
/// # fn main() {
/// #    (assert_c! {
/// # #include "tests/wasmer.h"
/// #
/// int main() {
///     // Create the engine and the store.
///     wasm_engine_t* engine = wasm_engine_new();
///     wasm_store_t* store = wasm_store_new(engine);
///
///     // Create a WebAssembly module from a WAT definition.
///     wasm_byte_vec_t wat;
///     wasmer_byte_vec_new_from_string(
///         &wat,
///         "(module\n"
///         "  (func (export \"function\") (param i32 i64))\n"
///         "  (global (export \"global\") i32 (i32.const 7))\n"
///         "  (table (export \"table\") 0 funcref)\n"
///         "  (memory (export \"memory\") 1))"
///     );
///     wasm_byte_vec_t wasm;
///     wat2wasm(&wat, &wasm);
///
///     // Create the module.
///     wasm_module_t* module = wasm_module_new(store, &wasm);
///
///     // Instantiate the module.
///     wasm_extern_vec_t imports = WASM_EMPTY_VEC;
///     wasm_trap_t* traps = NULL;
///
///     wasm_instance_t* instance = wasm_instance_new(store, module, &imports, &traps);
///     assert(instance);
///
///     // Read the exports.
///     wasm_extern_vec_t exports;
///     wasm_instance_exports(instance, &exports);
///
///     // We have 4 of them.
///     assert(exports.size == 4);
///
///     // The first one is a function. Use `wasm_extern_as_func`
///     // to go further.
///     assert(wasm_extern_kind(exports.data[0]) == WASM_EXTERN_FUNC);
///
///     // The second one is a global. Use `wasm_extern_as_global` to
///     // go further.
///     assert(wasm_extern_kind(exports.data[1]) == WASM_EXTERN_GLOBAL);
///
///     // The third one is a table. Use `wasm_extern_as_table` to
///     // go further.
///     assert(wasm_extern_kind(exports.data[2]) == WASM_EXTERN_TABLE);
///
///     // The fourth one is a memory. Use `wasm_extern_as_memory` to
///     // go further.
///     assert(wasm_extern_kind(exports.data[3]) == WASM_EXTERN_MEMORY);
///
///     // Free everything.
///     wasm_extern_vec_delete(&exports);
///     wasm_instance_delete(instance);
///     wasm_module_delete(module);
///     wasm_byte_vec_delete(&wasm);
///     wasm_byte_vec_delete(&wat);
///     wasm_store_delete(store);
///     wasm_engine_delete(engine);
///
///     return 0;
/// }
/// #    })
/// #    .success();
/// # }
/// ```
///
/// To go further:
///
/// * [`wasm_extern_as_func`][super::externals::wasm_extern_as_func],
/// * [`wasm_extern_as_global`][super::externals::wasm_extern_as_global],
/// * [`wasm_extern_as_table`][super::externals::wasm_extern_as_table],
/// * [`wasm_extern_as_memory`][super::externals::wasm_extern_as_memory].
#[no_mangle]
pub unsafe extern "C" fn wasm_instance_exports(
    instance: &wasm_instance_t,
    // own
    out: &mut wasm_extern_vec_t,
) {
    let instance = &instance.inner;
    let mut extern_vec = instance
        .exports
        .iter()
        .map(|(_name, r#extern)| Box::into_raw(Box::new(r#extern.clone().into())))
        .collect::<Vec<*mut wasm_extern_t>>();
    extern_vec.shrink_to_fit();

    out.size = extern_vec.len();
    out.data = extern_vec.as_mut_ptr();

    mem::forget(extern_vec);
}

#[cfg(test)]
mod tests {
    use inline_c::assert_c;

    #[test]
    fn test_instance_new() {
        (assert_c! {
            #include "tests/wasmer.h"

            // The `sum` host function implementation.
            wasm_trap_t* sum_callback(
                const wasm_val_vec_t* arguments,
                wasm_val_vec_t* results
            ) {
                wasm_val_t sum = {
                    .kind = WASM_I32,
                    .of = { arguments->data[0].of.i32 + arguments->data[1].of.i32 },
                };
                results->data[0] = sum;

                return NULL;
            }

            int main() {
                // Create the engine and the store.
                wasm_engine_t* engine = wasm_engine_new();
                wasm_store_t* store = wasm_store_new(engine);

                // Create a WebAssembly module from a WAT definition.
                wasm_byte_vec_t wat;
                wasmer_byte_vec_new_from_string(
                    &wat,
                    "(module\n"
                    "  (import \"math\" \"sum\" (func $sum (param i32 i32) (result i32)))\n"
                    "  (func (export \"add_one\") (param i32) (result i32)\n"
                    "    local.get 0\n"
                    "    i32.const 1\n"
                    "    call $sum))"
                );
                wasm_byte_vec_t wasm;
                wat2wasm(&wat, &wasm);

                // Create the module.
                wasm_module_t* module = wasm_module_new(store, &wasm);

                assert(module);

                // Prepare the imports.
                wasm_functype_t* sum_type = wasm_functype_new_2_1(
                    wasm_valtype_new_i32(),
                    wasm_valtype_new_i32(),
                    wasm_valtype_new_i32()
                );
                wasm_func_t* sum_function = wasm_func_new(store, sum_type, sum_callback);
                wasm_extern_t* externs[] = { wasm_func_as_extern(sum_function) };
                wasm_extern_vec_t imports = WASM_ARRAY_VEC(externs);

                // Instantiate the module.
                wasm_trap_t* traps = NULL;
                wasm_instance_t* instance = wasm_instance_new(store, module, &imports, &traps);

                assert(instance);

                // Run the exported function.
                wasm_extern_vec_t exports;
                wasm_instance_exports(instance, &exports);

                assert(exports.size == 1);

                const wasm_func_t* run_function = wasm_extern_as_func(exports.data[0]);

                assert(run_function);

                wasm_val_t arguments[1] = { WASM_I32_VAL(1) };
                wasm_val_t results[1] = { WASM_INIT_VAL };

                wasm_val_vec_t arguments_as_array = WASM_ARRAY_VEC(arguments);
                wasm_val_vec_t results_as_array = WASM_ARRAY_VEC(results);

                wasm_trap_t* trap = wasm_func_call(run_function, &arguments_as_array, &results_as_array);

                assert(trap == NULL);
                assert(results[0].of.i32 == 2);

                // Free everything.
                wasm_extern_vec_delete(&exports);
                wasm_instance_delete(instance);
                wasm_func_delete(sum_function);
                wasm_functype_delete(sum_type);
                wasm_module_delete(module);
                wasm_byte_vec_delete(&wasm);
                wasm_byte_vec_delete(&wat);
                wasm_store_delete(store);
                wasm_engine_delete(engine);

                return 0;
            }
        })
        .success();
    }
}
