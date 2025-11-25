#pragma once
#include "barretenberg/bbapi/bbapi_execute.hpp"
#include "barretenberg/serialize/cbind_fwd.hpp"
#include <vector>

namespace bb::bbapi {
// Function declaration for CLI usage
CommandResponse bbapi(Command&& command);
CommandResponse bbapi_non_chonk(Command&& command);
} // namespace bb::bbapi

// Forward declaration for CBIND
CBIND_DECL(bbapi)
WASM_EXPORT bool bbapi_non_chonk(const uint8_t* input_in, size_t input_len_in, uint8_t** output_out, size_t* output_len_out);

// Logging controls
extern "C" void bbapi_set_verbose_logging(bool enabled);
extern "C" void bbapi_set_debug_logging(bool enabled);

// Initialization and cleanup
extern "C" bool bbapi_init(const char* crs_path);
extern "C" void bbapi_cleanup();

