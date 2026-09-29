# Native throughput twin of symbol-rs/examples/bench.rs.
require "symbols"

CASES = [
  {"sum", false, "Σ [1, 2, 3, 4]"},
  {"constraint", false, "(x + y) > 10"},
  {"program", true, "x = 3. y = x * 2. Σ [x, y, 10]"},
  {"grade+index", false, "(⍋ [30, 10, 20, 50, 40]) @> [30, 10, 20, 50, 40]"},
  {"vector", false, "Σ [1, 2, 3, 4, 5] * [2, 3, 4, 5, 6]"},
  {"range", false, "Σ 1 .. 100"},
]

iterations = (ARGV[0]? || "200000").to_i
bindings = {"x" => 3_i64.as(SYMBOL::Tacit::TacitValue), "y" => 9_i64.as(SYMBOL::Tacit::TacitValue)}
sink = 0_i64
CASES.each do |(name, program, source)|
  start = Time.instant
  iterations.times do
    result = if program
               SYMBOL.eval(source, {} of String => SYMBOL::Tacit::TacitValue, program: true)
             else
               SYMBOL.eval(source, bindings)
             end
    sink &+= 1 if result.is_a?(SYMBOL::Tacit::Resolved)
  end
  ms = (Time.instant - start).total_milliseconds
  printf("%-12s %d evals  %8.1f ms  %7.0f ns/eval\n", name, iterations, ms, ms * 1e6 / iterations)
end
STDERR.puts "resolved: #{sink}"
