#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Authority {
    TrustedShell,
    Plugin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorityContext {
    authority: Authority,
}

impl AuthorityContext {
    #[must_use]
    pub const fn trusted_shell() -> Self {
        Self {
            authority: Authority::TrustedShell,
        }
    }

    #[must_use]
    pub const fn plugin() -> Self {
        Self {
            authority: Authority::Plugin,
        }
    }

    #[must_use]
    pub const fn authority(self) -> Authority {
        self.authority
    }
}
