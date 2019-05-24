use crate::gateway_grpc::*;
//use crate::gateway::*;

//use grpc::ClientStub;
use crate::gateway::{
    ActivateJobsRequest, CompleteJobRequest, CompleteJobResponse, CreateWorkflowInstanceResponse,
    DeployWorkflowRequest, DeployWorkflowResponse, ListWorkflowsResponse, PublishMessageRequest,
    TopologyResponse, WorkflowMetadata,
};
pub use crate::gateway::{
    ActivateJobsResponse, ActivatedJob, CreateWorkflowInstanceRequest, WorkflowRequestObject,
};
use grpc::ClientStubExt;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Gateway Error")]
    GatewayError(grpc::Error),
    #[fail(display = "Topology Error")]
    TopologyError(grpc::Error),
    #[fail(display = "List Workflows Error")]
    ListWorkflowsError(grpc::Error),
    #[fail(display = "Deploy Workflow Error")]
    DeployWorkflowError(grpc::Error),
    #[fail(display = "Create Workflow Instance Error")]
    CreateWorkflowInstanceError(grpc::Error),
    #[fail(display = "Activate Job Error")]
    ActivateJobError(grpc::Error),
    #[fail(display = "Complete Job Error")]
    CompleteJobError(grpc::Error),
    #[fail(display = "Publish Message Error")]
    PublishMessageError(grpc::Error),
}

pub struct Client {
    pub(crate) gateway_client: GatewayClient,
}

impl Client {
    pub fn new() -> Result<Self, Error> {
        let config = Default::default();
        let gateway_client = GatewayClient::new_plain("127.0.0.1", 26500, config)
            .map_err(|e| Error::GatewayError(e))?;
        Ok(Self { gateway_client })
    }

    /// Get the topology. The returned struct is similar to what is printed when running `zbctl status`.
    pub fn topology(&self) -> Result<TopologyResponse, Error> {
        let options = Default::default();
        let topology_request = Default::default();
        let grpc_response: grpc::SingleResponse<_> =
            self.gateway_client.topology(options, topology_request);
        let topology_response = grpc_response
            .wait_drop_metadata()
            .map_err(|e| Error::TopologyError(e))?;
        Ok(topology_response)
    }

    /// list the workflows
    pub fn list_workflows(&self) -> Result<Vec<WorkflowMetadata>, Error> {
        let options = Default::default();
        let list_workflows_request = Default::default();
        let grpc_response: grpc::SingleResponse<_> = self
            .gateway_client
            .list_workflows(options, list_workflows_request);
        let list_workflows_response: ListWorkflowsResponse = grpc_response
            .wait_drop_metadata()
            .map_err(|e| Error::ListWorkflowsError(e))?;
        let workflows: Vec<WorkflowMetadata> = list_workflows_response.workflows.into();
        Ok(workflows)
    }

    /// deploy a collection of workflows
    pub fn deploy_workflow(
        &self,
        workflow_requests: Vec<WorkflowRequestObject>,
    ) -> Result<DeployWorkflowResponse, Error> {
        let options = Default::default();
        let mut deploy_workflow_request = DeployWorkflowRequest::default();
        deploy_workflow_request.set_workflows(protobuf::RepeatedField::from(workflow_requests));
        let grpc_response: grpc::SingleResponse<_> = self
            .gateway_client
            .deploy_workflow(options, deploy_workflow_request);
        let deploy_workflow_response: DeployWorkflowResponse =
            grpc_response
                .wait_drop_metadata()
                .map_err(|e| Error::DeployWorkflowError(e))?;
        Ok(deploy_workflow_response)
    }

    /// create a workflow instance of latest version
    pub fn create_workflow_instance(
        &self,
        bpmn_process_id: String,
        payload: String,
    ) -> Result<CreateWorkflowInstanceResponse, Error> {
        let options = Default::default();
        let mut request = CreateWorkflowInstanceRequest::default();
        request.set_version(-1);
        request.set_bpmnProcessId(bpmn_process_id);
        request.set_payload(payload);
        let grpc_response: grpc::SingleResponse<_> = self
            .gateway_client
            .create_workflow_instance(options, request);
        let create_workflow_instance_response: CreateWorkflowInstanceResponse = grpc_response
            .wait_drop_metadata()
            .map_err(|e| Error::CreateWorkflowInstanceError(e))?;
        Ok(create_workflow_instance_response)
    }

    /// activate a job
    pub fn activate_job(
        &self,
        job_type: String,
        worker: String,
        timeout: i64,
        amount: i32,
    ) -> Vec<Result<Vec<ActivatedJob>, Error>> {
        let options = Default::default();
        let mut activate_jobs_request = ActivateJobsRequest::default();
        activate_jobs_request.set_amount(amount);
        activate_jobs_request.set_timeout(timeout);
        activate_jobs_request.set_worker(worker);
        activate_jobs_request.set_field_type(job_type);
        let grpc_response: grpc::StreamingResponse<_> = self
            .gateway_client
            .activate_jobs(options, activate_jobs_request);
        let results: Vec<_> = grpc_response
            .wait_drop_metadata()
            .as_mut()
            .map(|r| {
                r.map(|a| a.jobs.into_vec())
                    .map_err(|e| Error::ActivateJobError(e))
            })
            .collect();
        results
    }

    /// complete a job
    pub fn complete_job(
        &self,
        job_key: i64,
        payload: Option<String>,
    ) -> Result<CompleteJobResponse, Error> {
        let options = Default::default();
        let mut complete_job_request = CompleteJobRequest::default();
        complete_job_request.set_jobKey(job_key);
        if let Some(payload) = payload {
            complete_job_request.set_payload(payload);
        }
        let grpc_response: grpc::SingleResponse<_> = self
            .gateway_client
            .complete_job(options, complete_job_request);
        let result = grpc_response
            .wait_drop_metadata()
            .map_err(|e| Error::CompleteJobError(e));
        result
    }

    /// Publish a message
    pub fn publish_message(
        &self,
        name: String,
        correlation_key: String,
        time_to_live: i64,
        message_id: String,
        payload: String,
    ) -> Result<(), Error> {
        let options = Default::default();
        let mut publish_message_request = PublishMessageRequest::default();
        publish_message_request.set_payload(payload);
        publish_message_request.set_correlationKey(correlation_key);
        publish_message_request.set_messageId(message_id);
        publish_message_request.set_name(name);
        publish_message_request.set_timeToLive(time_to_live);
        let grpc_response: grpc::SingleResponse<_> = self
            .gateway_client
            .publish_message(options, publish_message_request);
        let result = grpc_response
            .wait_drop_metadata()
            .map(|_| ())
            .map_err(|e| Error::PublishMessageError(e));
        result
    }
}

#[cfg(test)]
mod tests {
    use crate::Client;

    #[test]
    fn check_topology() {
        let client = Client::new().unwrap();
        let topology = client.topology().unwrap();
        println!("{:?}", topology);
    }

    #[test]
    fn check_list_workflows() {
        let client = Client::new().unwrap();
        let workflows = client.list_workflows().unwrap();
        println!("{:?}", workflows);
    }
}