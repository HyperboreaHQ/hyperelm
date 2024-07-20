#[macro_export]
/// Implement boilerplate types and methods to the ClientApp trait.
/// 
/// ```rust
/// use std::sync::Arc;
/// 
/// use hyperelm::prelude::*;
/// use hyperelm::exports::*;
/// 
/// use hyperborealib::rest_api::prelude::*;
/// 
/// #[derive(serde::Serialize, serde::Deserialize)]
/// enum InReq { Ping }
/// 
/// #[derive(serde::Serialize, serde::Deserialize)]
/// enum InResp { Pong }
/// 
/// #[derive(serde::Serialize, serde::Deserialize)]
/// enum InMsg { Msg(String) }
/// 
/// #[derive(serde::Serialize, serde::Deserialize)]
/// enum OutReq { Ping }
/// 
/// #[derive(serde::Serialize, serde::Deserialize)]
/// enum OutResp { Pong }
/// 
/// #[derive(serde::Serialize, serde::Deserialize)]
/// enum OutMsg { Msg(String) }
/// 
/// hyperborealib::impl_as_json!(
///     InReq  InResp  InMsg
///     OutReq OutResp OutMsg
/// );
/// 
/// struct MyClientApp;
/// 
/// impl ClientApp for MyClientApp {
///     build_client!(
///         input: InReq => InResp, InMsg;
///         output: OutReq => OutResp, OutMsg;
/// 
///         client: hyperborealib::http::ReqwestHttpClient;
///         state: ();
///         error: ();
/// 
///         requests: {
///             InReq::Ping => |_, _| async {
///                 Ok(InResp::Pong)
///             }
///         };
/// 
///         messages: {
///             InMsg::Msg(msg) => |_, info: MessageInfo| async move {
///                 println!("Message: {msg}");
///                 println!("Sender: {}", info.sender.client.public_key.to_base64());
/// 
///                 Ok(())
///             }
///         };
///     );
/// 
///     fn get_params(&self) ->  &ClientAppParams {
///         todo!()
///     }
///
///     fn get_middlewire(&self) ->  &ClientMiddleware<Self::HttpClient>  {
///         todo!()
///     }
/// 
///     fn get_state(&self) -> Arc<Self::State> {
///         todo!()
///     }
/// }
/// ```
macro_rules! build_client {
    (input: $request:ty => $response:ty, $message:ty; $( $tail:tt )*) => {
        type InputRequest = $request;
        type InputResponse = $response;
        type InputMessage = $message;

        build_client!( $( $tail )* );
    };

    (output: $request:ty => $response:ty, $message:ty; $( $tail:tt )*) => {
        type OutputRequest = $request;
        type OutputResponse = $response;
        type OutputMessage = $message;

        build_client!( $( $tail )* );
    };

    (client: $client:ty; $( $tail:tt )*) => {
        type HttpClient = $client;

        build_client!( $( $tail )* );
    };

    (state: $state:ty; $( $tail:tt )*) => {
        type State = $state;

        build_client!( $( $tail )* );
    };

    (error: $error:ty; $( $tail:tt )*) => {
        type Error = $error;

        build_client!( $( $tail )* );
    };

    // Signatures generated by async-trait macro

    (requests: { $( $request:pat => $handler:expr )* }; $( $tail:tt )*) => {
        #[allow(unused_variables)]
        fn handle_request<'life0, 'async_trait>(
            &self,
            request: Self::InputRequest,
            info: $crate::exports::hyperborealib::rest_api::prelude::MessageInfo
        ) -> std::pin::Pin<Box<dyn std::future::Future<
            Output = Result<Self::InputResponse, $crate::client::ClientAppError<Self::Error>>
        > + Send + 'async_trait>>
        where
            'life0: 'async_trait,
            Self: 'async_trait
        {
            match request {
                $( $request => Box::pin(($handler)(self.get_state(), info)), )*

                #[allow(unreachable_patterns)]
                _ => unimplemented!()
            }
        }

        build_client!( $( $tail )* );
    };

    (messages: { $( $message:pat => $handler:expr )* }; $( $tail:tt )*) => {
        #[allow(unused_variables)]
        fn handle_message<'life0, 'async_trait>(
            &self,
            message: Self::InputMessage,
            info: $crate::exports::hyperborealib::rest_api::prelude::MessageInfo
        ) -> std::pin::Pin<Box<dyn std::future::Future<
            Output = Result<(), $crate::client::ClientAppError<Self::Error>>
        > + Send + 'async_trait>>
        where
            'life0: 'async_trait,
            Self: 'async_trait
        {
            match message {
                $( $message => Box::pin(($handler)(self.get_state(), info)), )*

                #[allow(unreachable_patterns)]
                _ => unimplemented!()
            }
        }

        build_client!( $( $tail )* );
    };

    () => {}
}