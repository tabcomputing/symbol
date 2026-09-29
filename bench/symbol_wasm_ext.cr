# Scratch-only: the unmodified Crystal WASM entry point plus a program-mode
# export mirroring symbol-rs's `symbol_eval_program` (fresh bindings per call).
require "symbol_wasm"
require "symbol/statement"

fun symbol_eval_program(input_ptr : Pointer(UInt8)) : Pointer(UInt8)
  input = String.new(input_ptr)
  result = begin
    bindings = {} of String => SYMBOL::Tacit::TacitValue
    SYMBOL.format_result(SYMBOL::StatementParser.eval_program(input, bindings))
  rescue ex
    "Error: #{ex.message}"
  end
  output_ptr = symbol_alloc(result.bytesize + 1)
  result.to_slice.copy_to(output_ptr, result.bytesize)
  output_ptr[result.bytesize] = 0_u8
  output_ptr
end
