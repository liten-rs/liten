use std::{
  future::Future,
  pin::Pin,
  task::{Context, Poll},
};

use thiserror::Error;

use crate::{context, sync::oneshot};

use super::{Task, TaskId};
