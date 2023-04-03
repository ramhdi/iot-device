use anyhow::Result;
use embedded_svc::mqtt::client::{Details::Complete, Event::Received, QoS};
use embedded_svc::wifi::{ClientConfiguration, Configuration, Wifi};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    mqtt::client::{EspMqttClient, EspMqttMessage, MqttClientConfiguration},
    nvs::EspDefaultNvsPartition,
    wifi::EspWifi,
};
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use log::{error, info, warn};
use std::{thread::sleep, time::Duration};

enum State {
    ConnWifi,
    ConnMqtt,
    Ok,
}

fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    println!("Starting app");

    let mut state: State;

    // Init peripherals, sys_loop, nvs
    let peripherals = Peripherals::take().unwrap();
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // Init wifi driver
    let mut wifi_driver = EspWifi::new(peripherals.modem, sys_loop, Some(nvs))?;

    const SSID: &str = "***";
    const PASSWORD: &str = "***";

    state = State::ConnWifi;

    wifi_driver.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: SSID.into(),
        password: PASSWORD.into(),
        ..Default::default()
    }))?;
    wifi_driver.start()?;

    // Init MQTT client
    let mqtt_config = MqttClientConfiguration::default();
    let broker_url = "mqtt://broker.hivemq.com:1883";

    let mut mqtt_client = EspMqttClient::new(broker_url, &mqtt_config, move |message_event| {
        warn!("Received from MQTT: {:?}", message_event)
    })?;

    // Publish message params
    let topic = "***";
    let qos = QoS::AtLeastOnce;
    let retain = true;
    let mut payload: &[u8];
    let mut i = 0;

    loop {
        match state {
            State::ConnWifi => {
                // Connect to wifi
                while !wifi_driver.is_connected()? {
                    // Try to connect until success
                    wifi_driver.connect()?;
                    let config = wifi_driver.get_configuration()?;
                    println!("Waiting for AP: {:?}", config);
                    sleep(Duration::from_secs(1));
                }
                println!("Connected to AP!");
                println!("IP info: {:?}", wifi_driver.sta_netif().get_ip_info()?);

                state = State::ConnMqtt;
            }
            State::ConnMqtt => {
                // mqtt_client = EspMqttClient::new(broker_url, &mqtt_config, move |message_event| {
                //     warn!("Received from MQTT: {:?}", message_event)
                // })?;
                println!("Connected to MQTT Broker!");

                state = State::Ok;
            }
            State::Ok => {
                // Check wifi connection
                if !wifi_driver.is_connected()? {
                    println!("Disconnected from AP!");
                    state = State::ConnWifi;
                } else {
                    // Publish to topic 'test/' every 10 seconds
                    i += 1;
                    let msg = format!("Hello MQTT! i = {}", i);
                    payload = msg.as_bytes();
                    mqtt_client.publish(topic, qos, retain, payload)?;
                    println!("Sent message to MQTT");
                    sleep(Duration::from_secs(10));
                }
            }
        }
    }

    // Ok(())
}
