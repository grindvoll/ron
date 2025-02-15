use serde_derive::{Deserialize, Serialize};

// GRCOV_EXCL_START
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    pub struct Extensions: usize {
        const UNWRAP_NEWTYPES = 0x1;
        const IMPLICIT_SOME = 0x2;
        const UNWRAP_VARIANT_NEWTYPES = 0x4;
    }
}
// GRCOV_EXCL_STOP

impl Extensions {
    /// Creates an extension flag from an ident.
    #[must_use]
    pub fn from_ident(ident: &str) -> Option<Extensions> {
        match ident {
            "unwrap_newtypes" => Some(Extensions::UNWRAP_NEWTYPES),
            "implicit_some" => Some(Extensions::IMPLICIT_SOME),
            "unwrap_variant_newtypes" => Some(Extensions::UNWRAP_VARIANT_NEWTYPES),
            _ => None,
        }
    }
}

impl Default for Extensions {
    fn default() -> Self {
        Extensions::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::Extensions;

    fn roundtrip_extensions(ext: Extensions) {
        let ron = crate::to_string(&ext).unwrap();
        let ext2: Extensions = crate::from_str(&ron).unwrap();
        assert_eq!(ext, ext2);
    }

    #[test]
    fn test_extension_serde() {
        roundtrip_extensions(Extensions::default());
        roundtrip_extensions(Extensions::UNWRAP_NEWTYPES);
        roundtrip_extensions(Extensions::IMPLICIT_SOME);
        roundtrip_extensions(Extensions::UNWRAP_VARIANT_NEWTYPES);
        roundtrip_extensions(Extensions::UNWRAP_NEWTYPES | Extensions::IMPLICIT_SOME);
        roundtrip_extensions(Extensions::UNWRAP_NEWTYPES | Extensions::UNWRAP_VARIANT_NEWTYPES);
        roundtrip_extensions(Extensions::IMPLICIT_SOME | Extensions::UNWRAP_VARIANT_NEWTYPES);
        roundtrip_extensions(
            Extensions::UNWRAP_NEWTYPES
                | Extensions::IMPLICIT_SOME
                | Extensions::UNWRAP_VARIANT_NEWTYPES,
        );
    }
}
