//! Tests for expected success and failure outputs from the R9k adapter for a
//! set of inputs captured as snapshots from the live system.

mod provider;

use std::fs::{self, File};

use r9k_adapter::{R9kMessage, SmarTrakEvent, StopInfo};
use serde::Deserialize;

// #[derive(Deserialize, Serialize)]
// enum TestResult {}

// struct TestCase {
//     request: R9kMessage,
// }

// Load each test case. For each, present the input to the adapter and compare
// the output expected.
#[tokio::test]
async fn run() {
    for entry in fs::read_dir("data/sessions2").expect("should read directory") {
        let file = File::open(entry.expect("should read entry").path()).expect("should open file");
        let _session: provider::Replay =
            serde_json::from_reader(&file).expect("should deserialize session");
    }
}

// A trait that expresses the structure of taking in some data and
// constructing (say by deserialization) an input and an output.
pub trait Fixture
{
    // Type of input data needed by the test case. In most cases this is likely
    // to be the request type of the handler under test.
    type Input;
    // Type of output data produced by the test case. This could be the
    // expected output type of the handler under test, or an error type for
    // failure cases. Many tests cases don't care about the handler's output
    // type but a type that represents success or failure of some internal
    // processing.
    type Output;
    // Type of error that can occur when producing the expected output.
    type Error;
    // Some handlers under test may require extension data in order to process
    // the input, say from configuration or external systems.
    type Extension: Default;
    // Sometimes the raw input data needs to be transformed before being
    // passed to the test case handler, for example to adjust timestamps to
    // be relative to 'now'.
    type TransformParams;

    // Convert input data into the input type needed by the test case handler.
    fn input(&self) -> Self::Input;

    // Convert input data into transformation parameters for the test case
    // handler.
    fn params(&self) -> Option<Self::TransformParams> {
        None
    }

    // Apply a transformation function to the input data before passing it to
    // the test case handler.
    fn transform<F>(&self, f: F) -> Self::Input
    where
        F: FnOnce(Self::Input, Option<Self::TransformParams>) -> Self::Input,
    {
        f(self.input(), self.params())
    }

    // Convert input data into extension data needed by the test case handler.
    fn extension(&self) -> Option<Self::Extension> {
        None
    }

    /// Convert input data into the expected output type needed by the test
    /// case handler, which could be an error for failure cases.
    ///
    /// # Errors
    ///
    /// Returns an error when the fixture cannot produce the expected output.
    fn output(&self) -> Option<Result<Self::Output, Self::Error>>;
}

pub struct TestCase<D> {
    data: D,
}

pub struct PreparedTestCase<D>
where
    D: Fixture
{
    pub input: D::Input,
    pub extension: Option<D::Extension>,
    pub output: Option<Result<D::Output, D::Error>>,
}

impl<D> TestCase<D>
where
    D: Clone + Fixture,
{
    #[must_use]
    pub const fn new(data: D) -> Self {
        Self { data }
    }

    pub fn prepare<F>(&self, transform: F) -> PreparedTestCase<D>
    where
        F: FnOnce(D::Input, Option<D::TransformParams>) -> D::Input,
        <D as Fixture>::Input: FnOnce(<D as Fixture>::Input, <D as Fixture>::TransformParams)
    {
        let input = self.data.transform(transform);
        let extension = self.data.extension();
        let output = self.data.output();
        PreparedTestCase { input, extension, output }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReplayData {
    pub input: String,
    pub params: Option<ReplayTransform>,
    pub extension: Option<ReplayExtension>,
    pub output: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReplayTransform {
    pub delay: i32,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReplayExtension {
    pub stop_info: StopInfo,
}

pub enum ReplayError {
    BadRequest {
        code: String,
        description: String,
    }
}

impl Fixture for ReplayData {
    type Input = R9kMessage;
    type Output = Option<Vec<SmarTrakEvent>>;
    type Error = ReplayError;
    type Extension = ReplayExtension;
    type TransformParams = ReplayTransform;

    fn input(&self) -> Self::Input {
        quick_xml::de::from_reader(self.input.as_bytes()).expect("should deserialize input")
    }

    fn params(&self) -> Option<Self::TransformParams> {
        self.params.clone()
    }

    fn extension(&self) -> Option<Self::Extension> {
        self.extension.clone()
    }

    fn output(&self) -> Option<Result<Self::Output, Self::Error>> {
        self.output.as_ref().map_or(Some(Ok(None)), |events| {
                let smartrak_events: Vec<SmarTrakEvent> = events
                    .iter()
                    .map(|e| {
                        serde_json::from_str(e).expect("should deserialize smartrak event")
                    })
                    .collect();
                Some(Ok(Some(smartrak_events)))
            })
    }
}
