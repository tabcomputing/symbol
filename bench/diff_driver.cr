# Differential driver: reads corpus lines "<E|P>\t<escaped source>" from a file,
# evaluates with a fixed set of bindings and prints a canonical, type-tagged
# rendering of each result, plus the WASM format_result string.
require "symbols"

alias TV = SYMBOL::Tacit::TacitValue

def esc(s : String) : String
  String.build do |io|
    s.each_char do |c|
      case c
      when '\\' then io << "\\\\"
      when '"'  then io << "\\\""
      else
        if c.ord < 0x20 || c.ord == 0x7f
          io << "\\x" << c.ord.to_s(16)
        else
          io << c
        end
      end
    end
  end
end

def unesc(s : String) : String
  String.build do |io|
    i = 0
    chars = s.chars
    while i < chars.size
      c = chars[i]
      if c == '\\' && i + 1 < chars.size
        i += 1
        case chars[i]
        when 'n' then io << '\n'
        when 't' then io << '\t'
        when 'r' then io << '\r'
        when 'v' then io << '\v'
        when '\\' then io << '\\'
        else io << '\\' << chars[i]
        end
      else
        io << c
      end
      i += 1
    end
  end
end

def canon_value(v : TV) : String
  case v
  when Int64   then "i:#{v}"
  when Float64 then "f:#{v}"
  when String  then "s:\"#{esc(v)}\""
  when Bool    then "b:#{v}"
  when Nil     then "nil"
  when Array   then "[" + v.map { |x| canon_value(x) }.join(",") + "]"
  else              "??"
  end
end

def canon_result(r : SYMBOL::Tacit::EvalResult) : String
  case r
  when SYMBOL::Tacit::Resolved  then "R " + canon_value(r.value)
  when SYMBOL::Tacit::Unbound   then "U #{r.name}"
  when SYMBOL::Tacit::Suspended then "S(#{r.op}/#{r.arity}|" + r.args.map { |a| canon_result(a) }.join(";") + ")"
  else                               "??"
  end
end

# Copy of SYMBOL.format_result from src/symbol_wasm.cr (that file can't be
# required alongside src/symbols.cr because both define SYMBOL::VERSION).
def format_result(result : SYMBOL::Tacit::EvalResult) : String
  case result
  when SYMBOL::Tacit::Resolved
    value = result.value
    case value
    when Int64
      value.to_s
    when Float64
      value.to_s
    when Array
      "[" + value.map(&.to_s).join(", ") + "]"
    else
      value.to_s
    end
  else
    result.to_s
  end
end

def base_bindings : Hash(String, TV)
  b = {} of String => TV
  b["x"] = 3_i64
  b["y"] = 2.5
  b["s"] = "hello"
  b["n"] = "42"
  b["xs"] = [1_i64, 2_i64, 3_i64] of SYMBOL::Tacit::TacitValue
  b["e"] = [] of SYMBOL::Tacit::TacitValue
  b["b"] = true
  b["f"] = false
  b["z"] = nil
  b["m"] = [1_i64, "a", [2_i64, 3.5] of SYMBOL::Tacit::TacitValue, true, nil, false] of SYMBOL::Tacit::TacitValue
  b["big"] = Int64::MAX
  b["w"] = "nan"
  b["nz"] = -0.0
  b["fs"] = [2.5, 1.0, 3.0] of SYMBOL::Tacit::TacitValue
  b["ns"] = ["3", "1", "2"] of SYMBOL::Tacit::TacitValue
  b["q"] = [1_i64, "nan", 2_i64, "nan", 0_i64, -1.5] of SYMBOL::Tacit::TacitValue
  b
end

File.each_line(ARGV[0]) do |line|
  next if line.empty?
  mode, _, raw = line.partition('\t')
  src = unesc(raw)
  bindings = base_bindings
  output = begin
    r = mode == "P" ? SYMBOL.eval(src, bindings, program: true) : SYMBOL.eval(src, bindings)
    fr = format_result(r).gsub(/0x[0-9a-f]+/, "0xADDR")
    s = canon_result(r) + " || \"" + esc(fr) + "\""
    if mode == "P"
      s += " || " + bindings.keys.sort.map { |k| "#{k}=#{canon_value(bindings[k])}" }.join(";")
    end
    s
  rescue ex
    "ERR #{esc(ex.message || "")}"
  end
  puts output
end
