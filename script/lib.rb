OPERAND_TYPE = {
  '$i8' => 'i8',
  '$u8' => 'u8',
  '$i32' => 'i32',
  '$u32' => 'u32',
  '$u64' => 'u64',
  '$label' => 'Self::Label',
  '$addr' => 'Self::Address'
}.freeze

def assert(condition, message)
  raise message unless condition
end

def load_instructions
  File.read('instructions.list').each_line.map(&:chomp)
end

def identifier(instruction)
  segments = instruction.split.map do |part|
    part = part.chomp(',')

    if part == '$label'
      nil
    elsif part[0] == '$'
      part[(1...part.length)]
    elsif part[0] == '['
      assert(part[-1] == ']', "missing ] in #{instruction}")
      'ptr_' + part[(1...-1)].gsub('+', '_plus_')
    else
      part.downcase
    end
  end

  segments.compact.join('_')
end

def operand_type(instruction)
  match = /\$([a-z0-9]+)/.match(instruction)
  match && OPERAND_TYPE.fetch(match[0])
end
