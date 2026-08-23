#!/usr/bin/env ruby
# frozen_string_literal: true

# Regenerates `fixtures/sooth_fixtures.json` by driving the real upstream
# `sooth` Ruby gem (https://github.com/kranzky/sooth -- authored by Jason
# Hutchens, Unlicensed) and capturing its observed/surprise/uncertainty
# outputs as the conformance oracle for `terraphim_sooth`.
#
# Requires:
#   gem install sooth   # native extension: needs a C toolchain (make, cc)
#
# Usage:
#   ruby scripts/generate_fixtures.rb > fixtures/sooth_fixtures.json
#
# Provenance header in the emitted JSON records Ruby + gem versions and the
# invocation command so a future reviewer can reproduce the capture bit-for-bit.
#
# Why this exists: Sooth is the symbol-prediction kernel used by MegaHAL.
# The Rust port in `crates/terraphim_sooth` is a faithful re-implementation of
# the C source (`ext/sooth_native/sooth_predictor.c`), but the conformance
# contract for the port is "match the gem's observable output bit-for-bit
# for the same observation history". This script produces that oracle.
#
# Upstream gem layout (verified against v2.3.2, 2022-03-23):
#   - `lib/sooth.rb` is a single line: `require 'sooth_native'`.
#   - `ext/sooth_native/native.c` exposes the `Sooth::Predictor` class with
#     `initialize(error_event)`, `observe(ctx, event)`, `count(ctx)`,
#     `size(ctx)`, `select(ctx, limit)`, `surprise(ctx, event)`,
#     `uncertainty(ctx)`, `frequency(ctx, event)`, `distribution(ctx)`,
#     `save(filename)`, `load(filename)`, `clear`.
#   - `ext/sooth_native/sooth_predictor.c` defines the math:
#       observe:     increments statistic.count + context.count
#       surprise:    -log2(statistic.count / context.count)
#       uncertainty: -sum(p * log2(p)) over the context distribution
#       select:      cumulative-count linear scan on the sorted statistic array
#     Contexts are single Fixnum ids (not a 2-tuple). terraphim_sooth's Rust
#     port extends the context to `(u32, u32)` for MegaHAL-style bigrams, but
#     the arithmetic is identical regardless of how the context key is
#     represented, so each scenario below uses one distinct scalar context
#     id (0, 1, 2, ...) -- the numbers don't depend on the key shape.
#   - `Marshal.dump(Predictor)` is NOT supported -- the gem uses its own
#     binary `MH11` save/load format. terraphim_sooth's Rust port therefore
#     uses serde JSON directly (see `Predictor::Serialize`/`Deserialize`).

require 'sooth'
require 'json'

# Five distinct scenarios; at least one context has >=3 symbols (so
# `uncertainty > 0`). Captured via the `Sooth::Predictor#observe` /
# `surprise` / `uncertainty` API.
SCENARIOS = [
  { name: 'skewed_three_one',         observe_sequence: [3, 3, 3, 7] },
  { name: 'uniform_pair',             observe_sequence: [1, 2] },
  { name: 'single_symbol_repeated',   observe_sequence: [5, 5, 5, 5, 5] },
  { name: 'four_way_uniform',         observe_sequence: [1, 2, 3, 4] },
  { name: 'skewed_five_way',          observe_sequence: [1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 3, 4, 5] }
].freeze

# Anything outside u32; `error_event` is required by the C API but never
# appears in any observed path for these scenarios.
ERROR_EVENT = 0xFFFF_FFFF

def run_scenario(context_id, scenario)
  predictor = Sooth::Predictor.new(ERROR_EVENT)
  observed_counts = scenario[:observe_sequence].map do |event|
    predictor.observe(context_id, event)
  end

  # Emit surprise only for symbols that were actually observed -- the gem
  # returns `nil` (which serialises to `null`) for unobserved symbols and
  # we deliberately don't include those in the fixture (it's test noise).
  symbols = scenario[:observe_sequence].uniq
  surprise = symbols.each_with_object({}) do |event, acc|
    acc[event.to_s] = predictor.surprise(context_id, event)
  end

  {
    name: scenario[:name],
    observe_sequence: scenario[:observe_sequence],
    expected_observe_counts: observed_counts,
    count: predictor.count(context_id),
    surprise: surprise,
    uncertainty: predictor.uncertainty(context_id)
  }
end

fixtures = {
  source: "Captured live by scripts/generate_fixtures.rb against the real " \
          "`sooth` Ruby gem (#{Gem::Specification.find_by_name('sooth').version}, " \
          "https://github.com/kranzky/sooth, Unlicense) on Ruby " \
          "#{RUBY_VERSION} (#{RUBY_PLATFORM}). Command: `ruby " \
          "crates/terraphim_sooth/scripts/generate_fixtures.rb`. " \
          "See crates/terraphim_sooth/scripts/generate_fixtures.rb for the " \
          "capture harness.",
  scenarios: SCENARIOS.each_with_index.map { |scenario, i| run_scenario(i, scenario) }
}

puts JSON.pretty_generate(fixtures)
