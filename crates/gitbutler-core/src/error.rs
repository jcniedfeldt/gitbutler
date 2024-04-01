//! ## How-To
//!
//! This is a primer on how to use the types provided here.
//!
//! Generally, if you do not care about attaching an error code for error-classification or
//! and/or messages that may show up in the user interface, read no further. Just use `anyhow`
//! or `thiserror` like before.
//!
//! ### Adding Context
//!
//! The [`Context`] type is the richest context we may attach to either `anyhow` errors or `thiserror`,
//! albeit using a different mechanism. This context is maintained as the error propagates and the
//! context higest up the error chain, the one most recently added, can be used by higher layers
//! of the GitButler application. Currently, a [`Context::message`] is shown in the user-interface,
//! whereas [`Context::code`] can be provided to help the user interface to make decisions, as it uses
//! the code for classifying errors.
//!
//! #### With `anyhow`
//!
//! The basis is an error without context, just by using `anyhow` in any way.
//!
//!```rust
//!# use anyhow::bail;
//! fn f() -> anyhow::Result<()> {
//!    bail!("internal information")
//! }
//!```
//!
//! Adding context is as easy as using the `context()` method on any `Result` or [`anyhow::Error`].
//! This can be a [`Code`], which automatically uses the message provided previously in the
//! frontend (note, though, that this is an implementation detail, which may change).
//! It serves as marker to make these messages show up, even if the code is [`Code::Unknown`].
//!
//!```rust
//!# use anyhow::{anyhow};
//!# use gitbutler_core::error::Code;
//! fn f() -> anyhow::Result<()> {
//!    return Err(anyhow!("user information").context(Code::Unknown))
//! }
//!```
//!
//! Finally, it's also possible to specify the user-message by using a [`Context`].
//!
//!```rust
//!# use anyhow::{anyhow};
//!# use gitbutler_core::error::{Code, Context};
//! fn f() -> anyhow::Result<()> {
//!    return Err(anyhow!("internal information").context(Context::new_static(Code::Unknown, "user information")))
//! }
//!```
//!
//! #### Backtraces and `anyhow`
//!
//! Backtraces are automatically collected when `anyhow` errors are instantiated, as long as the
//! `RUST_BACKTRACE` variable is set.
//!
//! #### With `thiserror`
//!
//! `thiserror` doesn't have a mechanism for generic context, which is why it has to be attached to
//! each type that is generated by thiserror.
//!
//! By default, `thiserror` instances have no context.
//!
//!```rust
//! # use gitbutler_core::error::{Code, Context, ErrorWithContext};
//! #[derive(thiserror::Error, Debug)]
//! #[error("user message")]
//! struct Error;
//!
//! // But context can be added like this:
//! impl ErrorWithContext for Error {fn context(&self) -> Option<Context> {
//!         // attach a code to make it show up in higher layers.
//!         Some(Context::from(Code::Unknown))
//!     }
//! }
//! ```
//!
//! Note that it's up to the implementation of [`ErrorWithContext`] to collect all context of errors in variants.
//!
//!```rust
//! # use gitbutler_core::error::{AnyhowContextExt, Code, Context, ErrorWithContext};
//!
//! #[derive(thiserror::Error, Debug)]
//! #[error("user message")]
//! struct TinyError;
//!
//! // But context can be added like this:
//! impl ErrorWithContext for TinyError {
//!     fn context(&self) -> Option<Context> {
//!         Some(Context::new_static(Code::Unknown, "tiny message"))
//!     }
//! }
//! #[derive(thiserror::Error, Debug)]
//! enum Error {
//!    #[error(transparent)]
//!    Tiny(#[from] TinyError),
//!    #[error(transparent)]
//!    Other(#[from] anyhow::Error)
//! };
//!
//! // But context can be added like this:
//! impl ErrorWithContext for Error {
//!     fn context(&self) -> Option<Context> {
//!        match self {
//!            Error::Tiny(err) => err.context(),
//!            Error::Other(err) => err.custom_context()
//!         }
//!     }
//! }
//! ```
//!
//! ### Assuring Context
//!
//! Currently, the consumers of errors with context are quite primitive and thus rely on `anyhow`
//! to collect and find context hidden in the error chain.
//! To make that work, it's important that `thiserror` based errors never silently convert into
//! `anyhow::Error`, as the context-consumers wouldn't find the context anymore.
//!
//! To prevent issues around this, make sure that relevant methods use the [`Error`] type provided
//! here. It is made to only automatically convert from types that have context information.
//! Those who have not will need to be converted by hand using [`Error::from_err()`].
use std::borrow::Cow;
use std::fmt::{Debug, Display};

/// A unique code that consumers of the API may rely on to identify errors.
#[derive(Debug, Default, Copy, Clone, PartialOrd, PartialEq)]
pub enum Code {
    #[default]
    Unknown,
    Validation,
    Projects,
    Branches,
    ProjectGitAuth,
    ProjectGitRemote,
    ProjectConflict,
    ProjectHead,
    Menu,
    PreCommitHook,
    CommitMsgHook,
}

impl std::fmt::Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = match self {
            Code::Menu => "errors.menu",
            Code::Unknown => "errors.unknown",
            Code::Validation => "errors.validation",
            Code::Projects => "errors.projects",
            Code::Branches => "errors.branches",
            Code::ProjectGitAuth => "errors.projects.git.auth",
            Code::ProjectGitRemote => "errors.projects.git.remote",
            Code::ProjectHead => "errors.projects.head",
            Code::ProjectConflict => "errors.projects.conflict",
            //TODO: rename js side to be more precise what kind of hook error this is
            Code::PreCommitHook => "errors.hook",
            Code::CommitMsgHook => "errors.hooks.commit.msg",
        };
        f.write_str(code)
    }
}

/// A context to wrap around lower errors to allow its classification, along with a message for the user.
#[derive(Default, Debug, Clone)]
pub struct Context {
    /// The identifier of the error.
    pub code: Code,
    /// A description of what went wrong, if available.
    pub message: Option<Cow<'static, str>>,
}

impl std::fmt::Display for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message.as_deref().unwrap_or("Something went wrong"))
    }
}

impl From<Code> for Context {
    fn from(code: Code) -> Self {
        Context {
            code,
            message: None,
        }
    }
}

impl Context {
    /// Create a new instance with `code` and an owned `message`.
    pub fn new(code: Code, message: impl Into<String>) -> Self {
        Context {
            code,
            message: Some(Cow::Owned(message.into())),
        }
    }

    /// Create a new instance with `code` and a statically known `message`.
    pub const fn new_static(code: Code, message: &'static str) -> Self {
        Context {
            code,
            message: Some(Cow::Borrowed(message)),
        }
    }
}

mod private {
    pub trait Sealed {}
}

/// A way to obtain attached Code or context information from `anyhow` contexts, so that
/// the more complete information is preferred.
pub trait AnyhowContextExt: private::Sealed {
    /// Return our custom context that might be attached to this instance.
    ///
    /// Note that it could not be named `context()` as this method already exists.
    fn custom_context(&self) -> Option<Context>;
}

impl private::Sealed for anyhow::Error {}
impl AnyhowContextExt for anyhow::Error {
    fn custom_context(&self) -> Option<Context> {
        if let Some(ctx) = self.downcast_ref::<Context>() {
            Some(ctx.clone())
        } else {
            self.downcast_ref::<Code>().map(|code| (*code).into())
        }
    }
}

/// A trait that if implemented on `thiserror` instance, allows to extract context we provide
/// in its variants.
///
/// Note that this is a workaround for the inability to control or implement the `provide()` method
/// on the `std::error::Error` implementation of `thiserror`.
pub trait ErrorWithContext: std::error::Error {
    /// Obtain the [`Context`], if present.
    fn context(&self) -> Option<Context>;
}

/// Convert `err` into an `anyhow` error, but also add provided `Code` or `Context` as anyhow context.
/// This uses the new `provide()` API to attach arbitrary information to error implementations.
pub fn into_anyhow(err: impl ErrorWithContext + Send + Sync + 'static) -> anyhow::Error {
    let context = err.context();
    let err = anyhow::Error::from(err);
    if let Some(context) = context {
        err.context(context)
    } else {
        err
    }
}

/// A wrapper around an `anyhow` error which automatically extracts the context that might be attached
/// to `thiserror` instances.
///
/// Whenever `thiserror` is involved, this error type should be used if the alternative would be to write
/// a `thiserror` which just forwards its context (like `app::Error` previously).
#[derive(Debug)]
pub struct Error(anyhow::Error);

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Self(value)
    }
}

impl From<Error> for anyhow::Error {
    fn from(value: Error) -> Self {
        value.0
    }
}

impl<E> From<E> for Error
where
    E: ErrorWithContext + Send + Sync + 'static,
{
    fn from(value: E) -> Self {
        Self(into_anyhow(value))
    }
}

impl Error {
    /// A manual, none-overlapping implementation of `From` (or else there are conflicts).
    pub fn from_err(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self(err.into())
    }

    /// Associated more context to the contained anyhow error
    pub fn context<C>(self, context: C) -> Self
    where
        C: Display + Send + Sync + 'static,
    {
        let err = self.0;
        Self(err.context(context))
    }

    /// Returns `true` if `E` is contained in our error chain.
    pub fn is<E>(&self) -> bool
    where
        E: Display + Debug + Send + Sync + 'static,
    {
        self.0.is::<E>()
    }

    /// Downcast our instance to the given type `E`, or `None` if it's not contained in our context or error chain.
    pub fn downcast_ref<E>(&self) -> Option<&E>
    where
        E: Display + Debug + Send + Sync + 'static,
    {
        self.0.downcast_ref::<E>()
    }
}
