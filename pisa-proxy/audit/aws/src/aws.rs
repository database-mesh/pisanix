// Copyright 2022 SphereEx Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_cloudwatchlogs::{model::InputLogEvent, Client};
use chrono::Utc;

pub struct CloudWatchLog {
    log_group_name: String,
    log_stream_name: String,
    timestamp: i64,
    message: String,
}

impl CloudWatchLog {
    pub fn new(group: String, stream: String, message: String) -> Self {
        let now = Utc::now();
        CloudWatchLog {
            log_group_name: group,
            log_stream_name: stream,
            timestamp: now.timestamp_millis(),
            message,
        }
    }
}

pub struct CloudWatchSinker {
    pub client: Client,
    // pub region: String,
}

impl CloudWatchSinker {
    pub async fn new() -> Self {
        let region_provider = RegionProviderChain::first_try("us-east-1");
        let shared_config = aws_config::from_env().region(region_provider).load().await;
        let client = Client::new(&shared_config);

        CloudWatchSinker { client }
    }
    pub async fn send(&self, input: CloudWatchLog) -> Result<(), aws_sdk_cloudwatchlogs::Error> {
        let log_group_name = input.log_group_name.clone();
        let log_stream_name = input.log_stream_name.clone();
        let message = input.message.clone();

        let streams =
            self.client.describe_log_streams().log_group_name(input.log_group_name).send().await?;
        for s in streams.log_streams().unwrap() {
            if s.log_stream_name().unwrap() == log_stream_name.clone() {
                let next = s.upload_sequence_token().unwrap();
                let builder = InputLogEvent::builder();
                let e = builder
                    .set_message(Some(message.clone()))
                    .set_timestamp(Some(input.timestamp))
                    .build();

                let resp = self
                    .client
                    .put_log_events()
                    .set_sequence_token(Some(next.to_string()))
                    .log_group_name(log_group_name.clone())
                    .log_stream_name(log_stream_name)
                    .log_events(e)
                    .send()
                    .await?;
                println!("aws resp {:?}", resp);
                break;
            }
        }
        Ok(())
    }
}
