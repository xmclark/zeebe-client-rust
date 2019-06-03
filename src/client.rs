use crate::activate_and_process_jobs::JobError;
use crate::complete_job::{complete_job, CompletedJobData};
pub use crate::gateway::{
    ActivateJobsResponse, ActivatedJob, CreateWorkflowInstanceRequest,
    CreateWorkflowInstanceResponse, DeployWorkflowRequest, DeployWorkflowResponse,
    ListWorkflowsResponse, PublishMessageRequest, TopologyResponse, WorkflowMetadata,
    WorkflowRequestObject,
};
use crate::gateway_grpc::*;
pub use crate::worker::Worker;
use futures::Future;
use grpc::ClientStubExt;
use std::sync::Arc;

use crate::create_workflow_instance::{
    create_workflow_instance_with_no_payload, create_workflow_instance_with_serializable_payload,
};
use crate::publish_message::{
    publish_message_with_no_payload, publish_message_with_serializable_payload,
};
use serde::Serialize;
#[cfg(feature = "timer")]
use std::time::Duration;
#[cfg(feature = "timer")]
use tokio::timer::Interval;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Gateway Error. {:?}", _0)]
    GatewayError(grpc::Error),
    #[fail(display = "Topology Error. {:?}", _0)]
    TopologyError(grpc::Error),
    #[fail(display = "List Workflows Error. {:?}", _0)]
    ListWorkflowsError(grpc::Error),
    #[fail(display = "Deploy Workflow Error. {:?}", _0)]
    DeployWorkflowError(grpc::Error),
    #[fail(display = "Create Workflow Instance Error. {:?}", _0)]
    CreateWorkflowInstanceError(grpc::Error),
    #[fail(display = "Activate Job Error. {:?}", _0)]
    ActivateJobError(grpc::Error),
    #[fail(display = "Complete Job Error. {:?}", _0)]
    CompleteJobError(grpc::Error),
    #[fail(display = "Publish Message Error. {:?}", _0)]
    PublishMessageError(grpc::Error),
    #[fail(display = "Fail Job Error. {:?}", _0)]
    FailJobError(grpc::Error),
    #[cfg(feature = "timer")]
    #[fail(display = "Interval Error. {:?}", _0)]
    IntervalError(tokio::timer::Error),
    #[fail(display = "Job Error. {:?}", _0)]
    JobError(JobError),
    #[fail(display = "Json Payload Serialization Error. {:?}", _0)]
    JsonError(serde_json::error::Error),
}

pub enum WorkflowVersion {
    Latest,
    Version(i32),
}

impl Into<i32> for WorkflowVersion {
    fn into(self) -> i32 {
        match self {
            WorkflowVersion::Latest => -1,
            WorkflowVersion::Version(v) => v,
        }
    }
}

#[derive(Clone)]
pub struct Client {
    pub(crate) gateway_client: Arc<GatewayClient>,
}

impl Client {
    /// Construct a new `Client` that connects to a broker with `host` and `port`.
    pub fn new<S: AsRef<str>>(host: S, port: u16) -> Result<Self, Error> {
        let config = Default::default();
        let gateway_client = Arc::new(
            GatewayClient::new_plain(host.as_ref(), port, config)
                .map_err(|e| Error::GatewayError(e))?,
        );
        Ok(Self { gateway_client })
    }

    /// Get the topology. The returned struct is similar to what is printed when running `zbctl status`.
    pub fn topology(&self) -> impl Future<Item = TopologyResponse, Error = Error> {
        let options = Default::default();
        let topology_request = Default::default();
        let grpc_response: grpc::SingleResponse<_> =
            self.gateway_client.topology(options, topology_request);
        grpc_response
            .drop_metadata()
            .map_err(|e| Error::TopologyError(e))
    }

    /// list the workflows
    pub fn list_workflows<I>(&self) -> impl Future<Item = Vec<WorkflowMetadata>, Error = Error> {
        let options = Default::default();
        let list_workflows_request = Default::default();
        let grpc_response: grpc::SingleResponse<ListWorkflowsResponse> = self
            .gateway_client
            .list_workflows(options, list_workflows_request);
        grpc_response
            .drop_metadata()
            .map(|r| r.workflows.into_vec())
            .map_err(|e| Error::ListWorkflowsError(e))
    }

    /// deploy a single bpmn workflow
    pub fn deploy_bpmn_workflow<S: Into<String>>(
        &self,
        workflow_name: S,
        workflow_definition: Vec<u8>,
    ) -> impl Future<Item = DeployWorkflowResponse, Error = Error> {
        let options = Default::default();
        let mut workflow_request_object = WorkflowRequestObject::default();

        // check for name ending in bpmn, and add it if missing
        let mut workflow_name = workflow_name.into();
        if !workflow_name.ends_with(".bpmn") {
            workflow_name.push_str(".bpmn");
        }

        workflow_request_object.set_name(workflow_name.into());
        workflow_request_object.set_definition(workflow_definition);
        let mut deploy_workflow_request = DeployWorkflowRequest::default();
        deploy_workflow_request
            .set_workflows(protobuf::RepeatedField::from(vec![workflow_request_object]));
        let grpc_response: grpc::SingleResponse<_> = self
            .gateway_client
            .deploy_workflow(options, deploy_workflow_request);
        grpc_response
            .drop_metadata()
            .map_err(|e| Error::DeployWorkflowError(e))
    }

    /// deploy a collection of workflows
    pub fn deploy_workflows(
        &self,
        workflow_requests: Vec<WorkflowRequestObject>,
    ) -> impl Future<Item = DeployWorkflowResponse, Error = Error> {
        let options = Default::default();
        let mut deploy_workflow_request = DeployWorkflowRequest::default();
        deploy_workflow_request.set_workflows(protobuf::RepeatedField::from(workflow_requests));
        let grpc_response: grpc::SingleResponse<_> = self
            .gateway_client
            .deploy_workflow(options, deploy_workflow_request);
        grpc_response
            .drop_metadata()
            .map_err(|e| Error::DeployWorkflowError(e))
    }

    /// create a workflow instance with a payload
    pub fn create_workflow_instance<'a, S: Into<String> + 'a, J: Serialize + 'a>(
        &'a self,
        bpmn_process_id: S,
        version: WorkflowVersion,
        value: J,
    ) -> impl Future<Item = CreateWorkflowInstanceResponse, Error = Error> + 'a {
        create_workflow_instance_with_serializable_payload(
            self.gateway_client.as_ref(),
            bpmn_process_id,
            version,
            value,
        )
    }
    /// create a workflow instance with no payload
    pub fn create_workflow_instance_no_payload<'a, S: Into<String> + 'a>(
        &'a self,
        bpmn_process_id: S,
        version: WorkflowVersion,
    ) -> impl Future<Item = CreateWorkflowInstanceResponse, Error = Error> + 'a {
        create_workflow_instance_with_no_payload(
            self.gateway_client.as_ref(),
            bpmn_process_id,
            version,
        )
    }

    //    /// activate jobs
    //    pub fn activate_jobs(
    //        &self,
    //        jobs_config: &ActivateJobsConfig,
    //    ) -> impl Stream<Item = gateway::ActivatedJob, Error = grpc::Error> + Send {
    //        activate_jobs(&self.gateway_client, &jobs_config)
    //    }

    /// complete a job
    pub fn complete_job(
        &self,
        completed_job_data: CompletedJobData,
    ) -> impl Future<Item = CompletedJobData, Error = grpc::Error> + Send {
        complete_job(&self.gateway_client, completed_job_data)
    }

    /// Publish a message with a payload
    pub fn publish_message<
        'a,
        S1: Into<String> + 'a,
        S2: Into<String> + 'a,
        S3: Into<String> + 'a,
        J: Serialize + 'a,
    >(
        &'a self,
        name: S1,
        correlation_key: S2,
        time_to_live: i64,
        message_id: S3,
        payload: J,
    ) -> impl Future<Item = (), Error = Error> + 'a {
        publish_message_with_serializable_payload(
            &self.gateway_client,
            name,
            correlation_key,
            time_to_live,
            message_id,
            payload,
        )
    }

    /// Publish a message without a payload
    pub fn publish_message_no_payload<
        'a,
        S1: Into<String> + 'a,
        S2: Into<String> + 'a,
        S3: Into<String> + 'a,
    >(
        &'a self,
        name: S1,
        correlation_key: S2,
        time_to_live: i64,
        message_id: S3,
    ) -> impl Future<Item = (), Error = Error> + 'a {
        publish_message_with_no_payload(
            &self.gateway_client,
            name,
            correlation_key,
            time_to_live,
            message_id,
        )
    }

    //    pub fn activate_and_process_jobs<F, X>(
    //        &self,
    //        worker_config: WorkerConfig,
    //        job_fn: JobFn<F,X>,
    //    ) -> impl Stream<Item = JobResult, Error = Error>
    //    where
    //        F: Fn(gateway::ActivatedJob) -> X + Send,
    //        X: IntoFuture<Item = JobResponse, Error = JobError>,
    //        <X as futures::future::IntoFuture>::Future: std::panic::UnwindSafe,
    //    {
    //        let client = self.gateway_client.clone();
    //        activate_and_process_jobs(client, worker_config, job_fn)
    //    }

    //    #[cfg(feature = "timer")]
    //    pub fn activate_and_process_jobs_interval<F, X>(
    //        &self,
    //        duration: Duration,
    //        worker_config: WorkerConfig,
    //        job_fn: JobFn<F,X>,
    //    ) -> impl Stream<Item = JobResult, Error = Error>
    //    where
    //        F: Fn(gateway::ActivatedJob) -> X + Send,
    //        X: IntoFuture<Item = JobResponse, Error = JobError>,
    //        <X as futures::future::IntoFuture>::Future: std::panic::UnwindSafe,
    //    {
    //        Interval::new_interval(duration)
    //            .map_err(|e| Error::IntervalError(e))
    //            .zip(futures::stream::repeat((
    //                self.gateway_client.clone(),
    //                worker_config,
    //                job_fn,
    //            )))
    //            .map(|(_, (gateway_client, worker_config, job_fn))| {
    //                activate_and_process_jobs(gateway_client, worker_config, job_fn)
    //            })
    //            .flatten()
    //    }

    pub fn worker<S: Into<String>>(&self, name: S) -> Worker {
        Worker::new(name, self.gateway_client.clone())
    }
}
