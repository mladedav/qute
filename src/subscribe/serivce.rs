// use mqttbytes::v5::Publish;
// use tower::Service;

// pub trait MqttService {
//     type Future;

//     fn process(&mut self, publish: Publish) -> Self::Future;
// }

// impl<T> MqttService for T
// where
//     T: Service<Publish>,
// {
//     type Future = T::Future;

//     fn process(&mut self, publish: Publish) -> Self::Future {
//         self.call(publish)
//     }
// }

// This does not work because I cannot make a blanket impl for foreign trait

// impl<T> Service<Publish> for T where T: MqttService {
//     type Response = ();
//     type Error = Infallible;
//     type Future = <T as MqttService>::Future;

//     fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         Poll::Ready(Ok(()))
//     }

//     fn call(&mut self, req: Publish) -> Self::Future {
//         self.process(req)
//     }
// }
