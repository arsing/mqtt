// Example:
//
//     cargo run --example publisher -- --server 127.0.0.1:1883 --client-id 'example-publisher' --topic foo --payload 'hello, world' --publish-frequency 1000

use futures::{ Future, Stream };

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

	#[structopt(help = "Maximum back-off time between reconnections to the server, in seconds.", long = "max-reconnect-back-off", default_value = "30")]
	max_reconnect_back_off: u64,

	#[structopt(help = "Keep-alive time advertised to the server, in seconds.", long = "keep-alive", default_value = "5")]
	keep_alive: u64,

	#[structopt(help = "How often to publish to the server, in milliseconds.", long = "publish-frequency", default_value = "1000")]
	publish_frequency: u64,

	#[structopt(help = "The topic of the publications.", long = "topic")]
	topic: String,

	#[structopt(help = "The QoS of the publications.", long = "qos", parse(from_str = "qos_from_str"))]
	qos: mqtt::proto::QoS,

	#[structopt(help = "The payload of the publications.", long = "payload")]
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
		publish_frequency,
		topic,
		qos,
		payload,
	} = structopt::StructOpt::from_args();

	let mut runtime = tokio::runtime::Runtime::new().expect("couldn't initialize tokio runtime");
	let executor = runtime.executor();

	let client =
		mqtt::Client::new(
			client_id,
			username,
			password,
			move || tokio::net::TcpStream::connect(&server),
			std::time::Duration::from_secs(max_reconnect_back_off),
			std::time::Duration::from_secs(keep_alive),
			10,
			10,
		);

	let mut publish_handle = client.publish_handle();
	let publish_loop =
		tokio::timer::Interval::new(std::time::Instant::now(), std::time::Duration::from_millis(publish_frequency))
		.then(move |result| {
			let _ = result.expect("timer failed");

			let topic = topic.clone();
			log::info!("Publishing to {} ...", topic);

			publish_handle
				.publish(mqtt::Publication {
					topic_name: topic.clone(),
					qos,
					retain: false,
					payload: payload.clone().into_bytes(),
				})
				.then(|result| {
					let () = result.expect("couldn't publish");
					Ok(topic)
				})
		})
		.for_each(|topic_name| {
			log::info!("Published to {}", topic_name);
			Ok(())
		});
	executor.spawn(publish_loop);

	let f = client.for_each(|_| Ok(()));

	runtime.block_on(f).expect("subscriber failed");
}

fn qos_from_str(s: &str) -> mqtt::proto::QoS {
	match s {
		"0" | "AtMostOnce" => mqtt::proto::QoS::AtMostOnce,
		"1" | "AtLeastOnce" => mqtt::proto::QoS::AtLeastOnce,
		"2" | "ExactlyOnce" => mqtt::proto::QoS::ExactlyOnce,
		s => panic!("unrecognized QoS {:?}: must be one of 0, 1, 2, AtMostOnce, AtLeastOnce, ExactlyOnce", s),
	}
}
