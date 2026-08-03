#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Authority {
    TrustedShell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorityContext {
    authority: Authority,
}

impl AuthorityContext {
    pub const fn trusted_shell() -> Self {
        Self {
            authority: Authority::TrustedShell,
        }
    }

    pub const fn authority(self) -> Authority {
        self.authority
    }
}
