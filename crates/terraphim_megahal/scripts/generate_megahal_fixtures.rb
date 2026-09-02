#!/usr/bin/env ruby
# frozen_string_literal: true

# Regenerates `fixtures/megahal_fixtures.json`: the conformance oracle for
# `terraphim_megahal` (terraphim/terraphim-ai#3261).
#
# The driver runs the REAL upstream MegaHAL logic from the vendored gem
# sources (`ruby_vendor/lib/megahal/*.rb`, kranzky/megahal, Unlicense) with
# the canonical RNG contract that the Rust port mirrors:
#
# 1. RNG = PCG32 (rand_pcg::Pcg32 / rand_core 0.10 semantics), mirrored in
#    pure Ruby below and pinned by the `rng_self_check` fixture vector.
# 2. `rand(n)` consumes exactly one `next_u32` and returns `value % n`
#    (`n == 0` returns 0). Upstream calls Kernel#rand; we override it.
# 3. `Array#shuffle` is a descending Fisher-Yates over that RNG
#    (`shuffle.first` is the canonical pick).
#
# Usage:
#   ruby scripts/generate_megahal_fixtures.rb > fixtures/megahal_fixtures.json
#
# Requires the `sooth` gem (gem install sooth -- native build). The `cld`
# native dependency of upstream is satisfied by the `ruby_vendor/cld.rb`
# stub (English-only fixtures).

require 'json'
require 'sooth'

vendor = File.expand_path('ruby_vendor', __dir__)
$LOAD_PATH.unshift(vendor)                 # cld.rb stub
$LOAD_PATH.unshift(File.join(vendor, 'lib'))

require 'cld'
require 'megahal/megahal.rb'
require 'megahal/keyword.rb'
require 'megahal/personalities.rb'

# ---------------------------------------------------------------------------
# Canonical RNG: PCG32 (XSH RR 64/32) mirroring rand_pcg::Pcg32 with the
# rand_core 0.10 `seed_from_u64` derivation.
# ---------------------------------------------------------------------------
class Pcg32
  MASK64 = 0xFFFF_FFFF_FFFF_FFFF
  MASK32 = 0xFFFF_FFFF
  MULTIPLIER = 6364136223846793005

  def self.rotate_right32(value, rot)
    ((value >> rot) | (value << (32 - rot))) & MASK32
  end

  # rand_core 0.10 `SeedableRng::seed_from_u64` default: derive the 16 seed
  # bytes from an internal PCG32 stream (4-byte little-endian chunks), then
  # `Pcg32::from_seed`.
  def self.seed_from_u64(seed)
    mul = 0x5851_F42D_4C95_7F2D
    inc = 0xA176_54E4_6FBE_17F3
    state = seed
    bytes = []
    4.times do
      state = (state * mul + inc) & MASK64
      xorshifted = (((state >> 18) ^ state) >> 27) & MASK32
      rot = (state >> 59) & 31
      x = rotate_right32(xorshifted, rot)
      bytes.concat([x].pack('V').bytes)
    end
    from_seed_bytes(bytes)
  end

  # rand_pcg 0.10 `Lcg64Xsh32::from_seed`: state = LE64(bytes[0..8]),
  # increment = LE64(bytes[8..16]) | 1 (from_state_incr takes the increment
  # directly; only the public `from_state` shifts), then:
  # state += increment, one step.
  def self.from_seed_bytes(bytes16)
    words = bytes16.pack('C*').unpack('Q< Q<')
    state = words[0]
    increment = words[1] | 1
    state = (state + increment) & MASK64
    state = (state * MULTIPLIER + increment) & MASK64
    new_from(state, increment)
  end

  def self.new_from(state, increment)
    rng = allocate
    rng.instance_variable_set(:@state, state)
    rng.instance_variable_set(:@increment, increment)
    rng
  end

  def initialize(seed)
    rng = Pcg32.seed_from_u64(seed)
    @state = rng.instance_variable_get(:@state)
    @increment = rng.instance_variable_get(:@increment)
  end

  def next_u32
    old = @state
    @state = (old * MULTIPLIER + @increment) & MASK64
    rot = old >> 59
    xsh = (((old >> 18) ^ old) >> 27) & MASK32
    Pcg32.rotate_right32(xsh, rot)
  end

  # Canonical rand(n): one draw always consumed; n == 0 yields 0.
  def below(n)
    value = next_u32
    n.zero? ? 0 : value % n
  end
end

# ---------------------------------------------------------------------------
# Canonical RNG patches over the vendored upstream logic.
# ---------------------------------------------------------------------------
$MEGAHAL_RNG = nil

def rand(n = 0)
  raise 'driver RNG not seeded' unless $MEGAHAL_RNG

  $MEGAHAL_RNG.below(n.is_a?(Integer) ? n : 0)
end

class Array
  # Canonical shuffle: descending Fisher-Yates. (`shuffle.first` is the
  # canonical "pick one at random".)
  def shuffle
    dup.canonical_shuffle!
  end

  def canonical_shuffle!
    (length - 1).downto(1) do |i|
      j = $MEGAHAL_RNG.below(i + 1)
      self[i], self[j] = self[j], self[i]
    end
    self
  end
end

# ---------------------------------------------------------------------------
# Scenario replay. Mirrors tests/conformance.rs exactly: one RNG seeded per
# scenario and threaded through the whole conversation.
# ---------------------------------------------------------------------------
def run_scenario(scenario)
  $MEGAHAL_RNG = Pcg32.new(scenario['seed'])

  hal = MegaHAL.new # trains the :default personality
  unless scenario['blank']
    hal.become(scenario['personality'].to_sym) if scenario['personality'] && scenario['personality'] != 'default'
  end
  if scenario['blank']
    hal.clear
  end
  hal.learning = scenario['learning']

  # Upstream has no public per-line learn; this is exactly what `train`
  # does with each line (strip -> decompose -> private _learn).
  scenario['train_lines'].each do |line|
    hal.send(:_learn, *hal.send(:_decompose, line.strip))
  end

  replies = scenario['conversation'].map do |input|
    hal.reply(input, '...')
  end

  { 'seed' => scenario['seed'], 'replies' => replies }
end

# Extra training corpora used by the scenarios (kept small: identical on both
# sides of the harness).
SCENARIO_CORPUS = [
  'I love Rust and WebAssembly.',
  "Don't repeat yourself, hob-goblin of bad code.",
  'Rust is a systems programming language.',
  'The Rust compiler is strict but fair.',
  'Markov chains are fun for chatting.',
  'A second-order Markov model predicts the next word.',
  'Terraphim builds search tools in Rust.',
  'The lazy dog sleeps all day while the quick fox jumps.'
].freeze

def scenarios
  [
    {
      'name' => 'rng_self_check',
      'seeds' => [42, 7, 2026]
    },
    {
      'name' => 'greeting_default_personality',
      'seed' => 42, 'personality' => 'default', 'blank' => false,
      'learning' => true, 'train_lines' => [],
      'conversation' => [nil, nil]
    },
    {
      'name' => 'keyword_reply_love',
      'seed' => 7, 'personality' => 'default', 'blank' => false,
      'learning' => true, 'train_lines' => SCENARIO_CORPUS,
      'conversation' => ['I love Rust.', 'What do you know about Markov chains?', 'Tell me about the fox.']
    },
    {
      'name' => 'learning_disabled',
      'seed' => 2026, 'personality' => 'default', 'blank' => false,
      'learning' => false, 'train_lines' => SCENARIO_CORPUS,
      'conversation' => ['Rust compiles fast.', 'Do you like Rust?']
    },
    {
      'name' => 'blank_brain_small_corpus',
      'seed' => 99, 'personality' => nil, 'blank' => true,
      'learning' => true, 'train_lines' => SCENARIO_CORPUS,
      'conversation' => ['Rust or WebAssembly?', 'Rust!', 'What is a Markov chain?']
    },
    {
      'name' => 'multi_turn_echo_guard',
      'seed' => 5150, 'personality' => 'default', 'blank' => false,
      'learning' => true, 'train_lines' => SCENARIO_CORPUS,
      'conversation' => ['Hello there.', 'I am a human being.', 'Time flies like an arrow.']
    },
    {
      'name' => 'empty_and_punctuated_input',
      'seed' => 8080, 'personality' => 'default', 'blank' => false,
      'learning' => true, 'train_lines' => [],
      'conversation' => ['', '!!!', 'don\'t stop']
    },
    {
      'name' => 'long_conversation_personality',
      'seed' => 31337, 'personality' => 'sherlock', 'blank' => false,
      'learning' => true, 'train_lines' => [],
      'conversation' => ['Hello Holmes.', 'What do you deduce?', 'Is Watson your friend?', 'Elementary, surely?']
    }
  ]
end

rng_self_check = [
  { 'seed' => 42, 'first_values' => (p = Pcg32.new(42); 8.times.map { p.next_u32 }) },
  { 'seed' => 7, 'first_values' => (p = Pcg32.new(7); 8.times.map { p.next_u32 }) },
  { 'seed' => 2026, 'first_values' => (p = Pcg32.new(2026); 8.times.map { p.next_u32 }) }
]

replayed = scenarios.map do |scenario|
  if scenario['name'] == 'rng_self_check'
    next nil
  end
  result = run_scenario(scenario)
  scenario.merge(result)
end.compact

fixtures = {
  'source' => "Captured by scripts/generate_megahal_fixtures.rb against the vendored upstream MegaHAL gem sources " \
              "(ruby_vendor/, kranzky/megahal, Unlicense) with the real `sooth` gem " \
              "(#{Gem::Specification.find_by_name('sooth').version}) on Ruby #{RUBY_VERSION} (#{RUBY_PLATFORM}). " \
              "Command: `ruby crates/terraphim_megahal/scripts/generate_megahal_fixtures.rb`. " \
              "Canonical RNG contract: PCG32 (rand_pcg::Pcg32 + rand_core 0.10 seed_from_u64, mirrored below), " \
              "rand(n) = next_u32 % n (n==0 -> 0, one draw always), Array#shuffle = descending Fisher-Yates.",
  'rng_self_check' => rng_self_check,
  'scenarios' => replayed
}

puts JSON.pretty_generate(fixtures)
