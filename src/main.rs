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

fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    println!("Entered Main function!");

    // Init peripherals, sys_loop, nvs
    let peripherals = Peripherals::take().unwrap();
    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    // Init wifi driver
    let mut wifi_driver = EspWifi::new(peripherals.modem, sys_loop, Some(nvs))?;

    const SSID: &str = "***";
    const PASSWORD: &str = "***";

    wifi_driver.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: SSID.into(),
        password: PASSWORD.into(),
        ..Default::default()
    }))?;

    // Connect to wifi
    wifi_driver.start().unwrap();
    wifi_driver.connect().unwrap();
    while !wifi_driver.is_connected()? {
        // Try to connect until success
        let config = wifi_driver.get_configuration()?;
        println!("Waiting for AP: {:?}", config);
        sleep(Duration::from_secs(1));
    }
    println!("Connected!");
    println!("IP info: {:?}", wifi_driver.sta_netif().get_ip_info()?);

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
        // Publish to topic 'test/' every 10 seconds
        i += 1;
        let msg = format!("Hello MQTT! i = {}", i);
        payload = msg.as_bytes();
        mqtt_client.publish(topic, qos, retain, payload)?;
        println!("Sent message to MQTT");
        sleep(Duration::from_secs(10));
    }

    // Ok(())
}
