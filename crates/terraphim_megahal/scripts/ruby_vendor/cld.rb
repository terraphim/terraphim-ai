# CLD stub for the terraphim_megahal fixture driver.
#
# Upstream megahal.rb `require 'cld'` uses the CLD native gem for language
# detection, falling back to character segmentation for CJK-like languages.
# The gem is not needed for conformance: every fixture conversation is
# English, so this stub always reports English (forcing word segmentation,
# the upstream default for alphabetic text).
module CLD
  def self.detect_language(_line)
    { name: 'ENGLISH' }
  end
end
