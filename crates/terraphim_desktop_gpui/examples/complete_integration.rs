fn main() {
    #[cfg(feature = "legacy-components")]
    {
        // Legacy example (feature-gated).
    }

    #[cfg(not(feature = "legacy-components"))]
    {
        // This example is only meaningful when the legacy component framework is enabled.
    }
}
