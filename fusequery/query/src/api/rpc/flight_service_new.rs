use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::channel;
use tonic::Request;
use tonic::Response as RawResponse;
use tonic::Status;
use tonic::Streaming;

use common_arrow::arrow_flight::Action;
use common_arrow::arrow_flight::ActionType;
use common_arrow::arrow_flight::Criteria;
use common_arrow::arrow_flight::Empty;
use common_arrow::arrow_flight::flight_service_server::FlightService;
use common_arrow::arrow_flight::FlightData;
use common_arrow::arrow_flight::FlightDescriptor;
use common_arrow::arrow_flight::FlightInfo;
use common_arrow::arrow_flight::HandshakeRequest;
use common_arrow::arrow_flight::HandshakeResponse;
use common_arrow::arrow_flight::PutResult;
use common_arrow::arrow_flight::SchemaResult;
use common_arrow::arrow_flight::Ticket;
use common_arrow::arrow_flight::Result as FlightResult;

use crate::api::rpc::flight_dispatcher::Request as DispatcherRequest;
use crate::api::rpc::FlightStream;
use tokio_stream::wrappers::ReceiverStream;
use common_exception::ErrorCodes;
use tokio_stream::StreamExt;

pub struct FuseQueryService {
    dispatcher_sender: Sender<DispatcherRequest>,
}

type Response<T> = Result<RawResponse<T>, Status>;
type StreamRequest<T> = Request<Streaming<T>>;

#[async_trait::async_trait]
impl FlightService for FuseQueryService {
    type HandshakeStream = FlightStream<HandshakeResponse>;

    async fn handshake(&self, request: StreamRequest<HandshakeRequest>) -> Response<Self::HandshakeStream> {
        unimplemented!()
    }

    type ListFlightsStream = FlightStream<FlightInfo>;

    async fn list_flights(&self, request: Request<Criteria>) -> Response<Self::ListFlightsStream> {
        unimplemented!()
        // let criteria = request.into_inner();
        // let expression = criteria.expression.into_string()?;
        //
        // fn get_flight(query_id: &String, stage_id: &String, flight_id: &String) -> FlightInfo {
        //     FlightInfo {
        //         schema:,
        //         endpoint: vec![],
        //         flight_descriptor: None,
        //         total_records: -1,
        //         total_bytes: -1,
        //     }
        // }
        //
        // fn get_flights(query_id: &String, v: (&String, &StagePtr), expressions: &Vec<&str>) -> Vec<FlightInfo> {
        //     let (stage_id, stage) = v;
        //     if expressions.len() < 3 {
        //         return stage.flights
        //             .iter()
        //             .map(|(id, _)| get_flight(query_id, stage_id, id))
        //             .collect_vec();
        //     }
        //
        //     stage.flights
        //         .iter()
        //         .filter(|(id, _)| id.starts_with(expressions[2]))
        //         .map(|(id, _)| get_flight(query_id, stage_id, id))
        //         .collect_vec()
        // }
        //
        // fn get_stage_flights(v: (&String, &QueryInfoPtr), expressions: &Vec<&str>) -> Vec<FlightInfo> {
        //     let (query_id, query) = v;
        //     if expressions.len() < 2 {
        //         return query.stages
        //             .iter()
        //             .flat_map(|v| get_flights(query_id, v, expressions))
        //             .collect_vec();
        //     }
        //
        //     query.stages
        //         .iter()
        //         .filter(|(id, _)| id.starts_with(expressions[1]))
        //         .flat_map(|v| get_flights(query_id, v, expressions))
        //         .collect_vec()
        // }
        //
        // fn get_queries_flights(queries: &Queries, expressions: &Vec<&str>) -> Vec<FlightInfo> {
        //     match expressions.len() {
        //         0 => {
        //             queries
        //                 .read()
        //                 .iter()
        //                 .flat_map(|v| get_stage_flights(v, expressions))
        //         },
        //         _ => {
        //             queries
        //                 .read()
        //                 .iter()
        //                 .filter(|(id, _)| id.starts_with(expressions[0]))
        //                 .flat_map(|v| get_stage_flights(v, expressions))
        //         }
        //     }.collect_vec()
        // }
        //
        // let expressions = expression.trim_start_matches("/").split("/").collect_vec();
        // let stream = futures::stream::iter(flightsget_queries_flights(&self.queries, &expressions).iter().map(Result::Ok));
        // Ok(Response::new(Box::pin(stream) as Self::DoActionStream))
    }

    async fn get_flight_info(&self, request: Request<FlightDescriptor>) -> Response<FlightInfo> {
        unimplemented!()
    }

    async fn get_schema(&self, request: Request<FlightDescriptor>) -> Response<SchemaResult> {
        unimplemented!()
    }

    type DoGetStream = FlightStream<FlightData>;

    async fn do_get(&self, request: Request<Ticket>) -> Response<Self::DoGetStream> {
        type DataReceiver = Receiver<common_exception::Result<FlightData>>;
        fn create_stream(receiver: DataReceiver) -> FlightStream<FlightData> {
            // TODO: Tracking progress is shown in the system.shuffles table
            Box::pin(ReceiverStream::new(receiver).map(|flight_data| {
                flight_data.map_err(|e| Status::internal(e.to_string()))
            })) as FlightStream<FlightData>
        }

        type ResultResponse = common_exception::Result<RawResponse<FlightStream<FlightData>>>;
        fn create_stream_response(receiver: Option<DataReceiver>) -> ResultResponse {
            receiver.ok_or_else(|| ErrorCodes::NotFoundStream("".to_string()))
                .map(create_stream).map(RawResponse::new)
        }

        match std::str::from_utf8(&request.into_inner().ticket) {
            Err(utf_8_error) => Err(Status::invalid_argument(utf_8_error.to_string())),
            Ok(ticket) => {
                // Flight ticket = query_id/stage_id/stream_id
                println!("Get Flight Stream {}", ticket);
                let (response_sender, mut receiver) = channel(1);
                self.dispatcher_sender.send(DispatcherRequest::GetStream(ticket.to_string(), response_sender)).await;
                receiver.recv().await
                    .transpose()
                    .and_then(create_stream_response)
                    .map_err(|e| Status::internal(e.to_string()))
            }
        }
    }

    type DoPutStream = FlightStream<PutResult>;

    async fn do_put(&self, _: StreamRequest<FlightData>) -> Response<Self::DoPutStream> {
        Result::Err(Status::unimplemented("FuseQuery does not implement do_put."))
    }

    type DoExchangeStream = FlightStream<FlightData>;

    async fn do_exchange(&self, _: StreamRequest<FlightData>) -> Response<Self::DoExchangeStream> {
        Result::Err(Status::unimplemented("FuseQuery does not implement do_exchange."))
    }

    type DoActionStream = FlightStream<FlightResult>;

    async fn do_action(&self, request: Request<Action>) -> Response<Self::DoActionStream> {
        unimplemented!()
    }

    type ListActionsStream = FlightStream<ActionType>;

    async fn list_actions(&self, request: Request<Empty>) -> Response<Self::ListActionsStream> {
        unimplemented!()
    }
}

