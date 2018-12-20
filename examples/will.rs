// This example demonstrates the use of a will.
//
// The client will connect to the server with a will built from the --topic, --qos and --payload parameters. It will then subscribe to the same topic.
// If the client receives a Ctrl-C, it will exit without properly shutting down the client. Thus the client will not be able to send a DISCONNECT
// to the server, so the server will publish the will to all subscribers.
//
// To demonstrate the effect, run two or more instances of this example with different client IDs (and optionally, different QoS and payloads)
// but the same topic subscription, then kill one with Ctrl-C. The other instances should all receive the will.
//
// Example:
//
//     cargo run --example will -- --server 127.0.0.1:1883 --client-id 'example-will-1' --topic foo --qos 1 --payload '"goodbye, world"  - example-will-1'
//     cargo run --example will -- --server 127.0.0.1:1883 --client-id 'example-will-2' --topic foo --qos 1 --payload '"goodbye, world"  - example-will-2'

use futures::{ Future, Stream };

mod common;

#[derive(Debug, structopt_derive::StructOpt)]
struct Options {
	#[structopt(help = "Address of the MQTT server.", long = "server")]
	server: std::net::SocketAddr,

	#[structopt(help = "Client ID used to identify this application to the server. If not given, a server-generated ID will be used.", long = "client-id")]
	client_id: Option<String>,

	#[structopt(help = "Username used to authenticate with the server, if any.", long = "username")]
	username: Option<String>,

	#[structopt(help = "Password used to authenticate with the server, if any.", long = "password")]
	password: Option<String>,

	#[structopt(
		help = "Maximum back-off time between reconnections to the server, in seconds.",
		long = "max-reconnect-back-off",
		default_value = "30",
		parse(try_from_str = "common::duration_from_secs_str"),
	)]
	max_reconnect_back_off: std::time::Duration,

	#[structopt(
		help = "Keep-alive time advertised to the server, in seconds.",
		long = "keep-alive",
		default_value = "5",
		parse(try_from_str = "common::duration_from_secs_str"),
	)]
	keep_alive: std::time::Duration,

	#[structopt(help = "The topic of the will.", long = "topic")]
	topic: String,

	#[structopt(help = "The QoS of the will.", long = "qos", parse(try_from_str = "common::qos_from_str"))]
	qos: mqtt::proto::QoS,

	#[structopt(help = "The payload of the will.", long = "payload")]
	payload: String,
}

fn main() {
	env_logger::Builder::from_env("MQTT_LOG").init();

	let Options {
		server,
		client_id,
		username,
		password,
		max_reconnect_back_off,
		keep_alive,
		topic,
		qos,
		payload,
	} = structopt::StructOpt::from_args();

	let mut runtime = tokio::runtime::Runtime::new().expect("couldn't initialize tokio runtime");

	let will = mqtt::proto::Publication {
		topic_name: topic.clone(),
		qos,
		retain: false,
		payload: payload.into_bytes(),
	};

	let client =
		mqtt::Client::new(
			client_id,
			username,
			password,
			Some(will),
			move || tokio::net::TcpStream::connect(&server),
			max_reconnect_back_off,
			keep_alive,
		);

	let mut update_subscription_handle = client.update_subscription_handle().expect("couldn't get subscription update handle");;
	runtime.spawn(
		update_subscription_handle
		.subscribe(mqtt::proto::SubscribeTo {
			topic_filter: topic,
			qos,
		})
		.map_err(|err| panic!("couldn't update subscription: {}", err)));

	let f = client.for_each(|event| {
		if let mqtt::Event::Publication(publication) = event {
			match std::str::from_utf8(&publication.payload) {
				Ok(s) =>
					log::info!(
						"Received publication: {:?} {:?} {:?}",
						publication.topic_name,
						s,
						publication.qos,
					),
				Err(_) =>
					log::info!(
						"Received publication: {:?} {:?} {:?}",
						publication.topic_name,
						publication.payload,
						publication.qos,
					),
			}
		}

		Ok(())
	});

	runtime.block_on(f).expect("will failed");
}
