//! Python bindings and RPC bridge for controller implementations.
//!
//! This module connects Python controller objects to the Rust simulation runtime.
//! [`PythonController`] owns a Python model and serves asynchronous RPC requests from
//! [`PythonControllerAsyncClient`].
//!
//! It also exposes [`ControllerWrapper`], the Python-visible base class used for custom
//! controller implementations.

use std::{str::FromStr, sync::Arc};

use pyo3::{prelude::*, types::PyDict};
use serde_json::Value;
use simusa_com::rfc::{self, RemoteFunctionCall, RemoteFunctionCallHost};

use simusa::{
    context::Context, errors::SimusaResult, internal, node::Node, physics::robot_models::Command,
    recordable::Recordable,
};

use crate::utils::{call_py_method, call_py_method_void};

use crate::wrappers::{CommandWrapper, ControllerErrorWrapper, NodeWrapper};
use simusa::controllers::{Controller, ControllerError, ControllerRecord};

#[derive(Debug, Clone)]
/// Simulator-side RPC handles used by the runtime to call Python controller methods.
pub struct PythonControllerAsyncClient {
    post_init: RemoteFunctionCall<NodeWrapper, SimusaResult<()>>,
    make_command: RemoteFunctionCall<(NodeWrapper, ControllerError, f32), Command>,
    record: RemoteFunctionCall<(), ControllerRecord>,
    pre_loop_hook: RemoteFunctionCall<(NodeWrapper, f32), ()>,
    next_time_step: RemoteFunctionCall<(), Option<f32>>,
}

impl Controller for PythonControllerAsyncClient {
    fn post_init(&mut self, node: &mut Node, context: &Context) -> SimusaResult<()> {
        let node_py = NodeWrapper::from_rust(node, context.clone());
        self.post_init.call(node_py).unwrap()
    }

    fn make_command(
        &mut self,
        node: &mut Node,
        error: &ControllerError,
        time: f32,
        context: &Context,
    ) -> Command {
        let node_py = NodeWrapper::from_rust(node, context.clone());
        self.make_command
            .call((node_py, error.clone(), time))
            .unwrap()
    }

    fn pre_loop_hook(&mut self, node: &mut Node, time: f32, context: &Context) {
        let node_py = NodeWrapper::from_rust(node, context.clone());
        self.pre_loop_hook.call((node_py, time)).unwrap()
    }

    fn next_time_step(&self, _context: &Context) -> Option<f32> {
        self.next_time_step.call(()).unwrap()
    }
}

impl Recordable<ControllerRecord> for PythonControllerAsyncClient {
    fn record(&self, _context: &Context) -> ControllerRecord {
        self.record.call(()).unwrap()
    }
}

#[derive(Debug)]
/// Python-side bridge that executes controller methods on a Python model.
pub struct PythonController {
    model: Py<PyAny>,
    client: PythonControllerAsyncClient,
    post_init: Arc<RemoteFunctionCallHost<NodeWrapper, SimusaResult<()>>>,
    make_command: Arc<RemoteFunctionCallHost<(NodeWrapper, ControllerError, f32), Command>>,
    record: Arc<RemoteFunctionCallHost<(), ControllerRecord>>,
    pre_loop_hook: Arc<RemoteFunctionCallHost<(NodeWrapper, f32), ()>>,
    next_time_step: Arc<RemoteFunctionCallHost<(), Option<f32>>>,
}

impl PythonController {
    /// Creates a new Python controller bridge.
    ///
    /// The provided `py_model` must implement the [`Controller`] trait.
    pub fn new(py_model: Py<PyAny>) -> PythonController {
        let (post_init_client, post_init_host) = rfc::make_pair();
        let (make_command_client, make_command_host) = rfc::make_pair();
        let (record_client, record_host) = rfc::make_pair();
        let (pre_loop_hook_client, pre_loop_hook_host) = rfc::make_pair();
        let (next_time_step_client, next_time_step_host) = rfc::make_pair();

        PythonController {
            model: py_model,
            client: PythonControllerAsyncClient {
                post_init: post_init_client,
                make_command: make_command_client,
                record: record_client,
                pre_loop_hook: pre_loop_hook_client,
                next_time_step: next_time_step_client,
            },
            post_init: Arc::new(post_init_host),
            make_command: Arc::new(make_command_host),
            record: Arc::new(record_host),
            next_time_step: Arc::new(next_time_step_host),
            pre_loop_hook: Arc::new(pre_loop_hook_host),
        }
    }
}

impl PythonController {
    /// Returns a cloneable asynchronous client used by the runtime.
    pub fn get_client(&self) -> PythonControllerAsyncClient {
        self.client.clone()
    }

    /// Polls pending RPC calls and dispatches them to Python implementations.
    ///
    /// ## Arguments
    /// * `context` - Shared simulation context used for logging and call tracing.
    pub fn check_requests(&mut self, context: &Context) {
        self.post_init
            .clone()
            .try_recv_closure_mut(|node| self.post_init(node, context));
        self.make_command
            .clone()
            .try_recv_closure_mut(|(node, error, time)| {
                self.make_command(node, &error, time, context)
            });
        self.record.try_recv_closure(|()| self.record(context));
        self.pre_loop_hook
            .clone()
            .try_recv_closure_mut(|(node, time)| self.pre_loop_hook(node, time, context));
        self.next_time_step
            .clone()
            .try_recv_closure(|()| self.next_time_step(context));
    }

    fn post_init(&mut self, node: NodeWrapper, context: &Context) -> SimusaResult<()> {
        internal!(
            context,
            ::simusa::logger::InternalLog::API,
            "Calling python implementation of post_init"
        );
        call_py_method_void!(self.model, "post_init", (node,));
        Ok(())
    }

    fn make_command(
        &mut self,
        node: NodeWrapper,
        error: &ControllerError,
        time: f32,
        context: &Context,
    ) -> Command {
        internal!(
            context,
            ::simusa::logger::InternalLog::API,
            "Calling python implementation of make_command"
        );
        // let node_record = node.record();
        let result = call_py_method!(
            self.model,
            "make_command",
            CommandWrapper,
            node,
            ControllerErrorWrapper::from_rust(error),
            time
        );
        result.to_rust()
    }

    fn record(&self, context: &Context) -> ControllerRecord {
        internal!(
            context,
            ::simusa::logger::InternalLog::API,
            "Calling python implementation of record"
        );
        let record_str: String = call_py_method!(self.model, "record", String,);
        let record = Value::from_str(&record_str).expect(
            "Impossible to get serde_json::Value from the input serialized python structure",
        );
        ControllerRecord::Plugin(record)
    }

    fn pre_loop_hook(&mut self, node: NodeWrapper, time: f32, context: &Context) {
        internal!(
            context,
            ::simusa::logger::InternalLog::API,
            "Calling python implementation of pre_loop_hook"
        );
        call_py_method_void!(self.model, "pre_loop_hook", node, time);
    }

    fn next_time_step(&self, context: &Context) -> Option<f32> {
        internal!(
            context,
            ::simusa::logger::InternalLog::API,
            "Calling python implementation of next_time_step"
        );
        call_py_method!(self.model, "next_time_step", Option<f32>,)
    }
}

#[pyclass(subclass)]
#[pyo3(name = "Controller")]
/// Python-visible base class for custom controller implementations.
///
/// Python subclasses are expected to override the exposed methods.
pub struct ControllerWrapper {}

#[pymethods]
impl ControllerWrapper {
    /// Creates a new Python controller wrapper.
    ///
    /// The constructor signature matches simulator expectations.
    #[new]
    pub fn new(_config: Py<PyAny>, _initial_time: f32) -> ControllerWrapper {
        Self {}
    }

    fn post_init(&mut self, _node: NodeWrapper) {}

    fn make_command(
        &mut self,
        _node: NodeWrapper,
        _error: ControllerErrorWrapper,
        _time: f32,
    ) -> CommandWrapper {
        unimplemented!()
    }

    fn record(&self) -> Py<PyDict> {
        unimplemented!()
    }

    fn pre_loop_hook(&mut self, _node: NodeWrapper, _time: f32) {
        unimplemented!()
    }

    fn next_time_step(&self) -> Option<f32> {
        None
    }
}
