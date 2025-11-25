#include "c_bind.hpp"
#include "barretenberg/bbapi/bbapi_execute.hpp"
#include "barretenberg/bbapi/bbapi_shared.hpp"
#include "barretenberg/common/log.hpp"
#include "barretenberg/common/throw_or_abort.hpp"
#include "barretenberg/serialize/msgpack_impl.hpp"
#include "barretenberg/srs/global_crs.hpp"
#ifndef NO_MULTITHREADING
#include <mutex>
#endif

namespace bb::bbapi {

// Global BBApiRequest object in anonymous namespace
namespace {
// NOLINTNEXTLINE(cppcoreguidelines-avoid-non-const-global-variables)
BBApiRequest global_request;
} // namespace

/**
 * @brief Main API function that processes commands and returns responses
 *
 * @param command The command to execute.
 * @return CommandResponse The response from executing the command
 */
CommandResponse bbapi(Command&& command)
{
    // Execute the command using the global request and return the response
    return execute(global_request, std::move(command));
}

/**
 * @brief API function that processes non-chonk commands and returns responses
 *
 * @param command The command to execute. Must not be a Chonk command.
 * @return CommandResponse The response from executing the command
 */
CommandResponse bbapi_non_chonk(Command&& command)
{
    try {
        // Check if this is a Chonk command
        if (is_chonk_command(command)) {
            return ErrorResponse{ .message =
                                      "Chonk commands are not supported in bbapi_non_chonk. Use bbapi instead." };
        }
        // Execute the command using the global request and return the response
        BBApiRequest request{};
        return execute(request, std::move(command));
    } catch (const std::exception& e) {
        return ErrorResponse{ .message = std::string("Exception during bbapi_non_chonk execution: ") + e.what() };
    } catch (...) {
        return ErrorResponse{ .message = "Unknown exception during bbapi_non_chonk execution." };
    }
}

} // namespace bb::bbapi

// Use CBIND macro to export the bbapi function for WASM
CBIND_WRAPPED_NOSCHEMA(bbapi_non_chonk, bb::bbapi::bbapi_non_chonk)
CBIND_NOSCHEMA(bbapi, bb::bbapi::bbapi)

extern "C" void bbapi_set_verbose_logging(bool enabled)
{
    verbose_logging = enabled;
}

extern "C" void bbapi_set_debug_logging(bool enabled)
{
    debug_logging = enabled;
}

namespace bb::srs {
extern "C" bool bbapi_init(const char* crs_path)
{
    try {
        // Initialize the CRS factory
        // This sets up the global factory that loads/downloads SRS points on-demand
        if (crs_path == nullptr) {
            // Use default path
            init_net_crs_factory(bb_crs_path());
        } else {
            init_net_crs_factory(std::string(crs_path));
        }
        return true;
    } catch (const std::exception& e) {
        // Log the error using Barretenberg's logging system
        info("Failed to initialize Barretenberg: ", e.what());
        return false;
    } catch (...) {
        // Catch any other exceptions
        info("Failed to initialize Barretenberg: unknown error");
        return false;
    }
}

extern "C" void bbapi_cleanup()
{
    // Clean up any global state if needed
    // Currently, there's not much to clean up
}
} // namespace bb::srs
