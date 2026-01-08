//! Tests for expected success and failure outputs from the R9k adapter for a
//! set of inputs captured as snapshots from the live system.

mod provider;

use std::fs::{self, File};

use r9k_adapter::StopInfo;
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

// A trait that expresses the ability to transform some input data I using
// transformation parameters T. The default implementation is a no-op.
pub trait Transformer<I, T> {
    fn transform<F>(&self, input: I, _: F) -> I
    where
        F: FnOnce(I, T) -> I,
    {
        input
    }
}

impl<I> Transformer<I, ()> for () {}

// A trait that expresses the structure of taking in some data and
// constructing (say by deserialization) an input and an output. Optionally, a
// transform function can be provided to modify the input before processing. The
// transform function can also take parameters, which can be provided by the
// generic parameter T. This defaults to the unit type if not specified.
//
// Similarly the handler under test may require some extension data to be
// provided in order to process the input. This is expressed by the generic
// parameter E.
pub trait Fixture<T = ()>
where
    T: Transformer<Self::Input, T>,
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

    // Convert input data into the input type needed by the test case handler.
    fn input(&self) -> Self::Input;

    // Convert input data into transformation parameters for the test case
    // handler.
    fn params(&self) -> T
    where
        T: Default,
    {
        T::default()
    }

    // Convert input data into extension data needed by the test case handler.
    fn extension(&self) -> Self::Extension {
        Self::Extension::default()
    }

    /// Convert input data into the expected output type needed by the test
    /// case handler, which could be an error for failure cases.
    ///
    /// # Errors
    ///
    /// Returns an error when the fixture cannot produce the expected output.
    fn output(&self) -> Result<Self::Output, Self::Error>;
}

pub struct TestCase<D, T> {
    data: D,
    _phantom: std::marker::PhantomData<T>,
}

pub struct PreparedTestCase<D, T = ()>
where
    D: Fixture<T>, T: Transformer<<D as Fixture<T>>::Input, T>
{
    pub input: D::Input,
    pub output: Result<D::Output, D::Error>,
}

impl<D, T> TestCase<D, T>
where
    D: Clone + Fixture<T>,
    T: Transformer<<D as Fixture<T>>::Input, T>
{
    #[must_use]
    pub const fn new(data: D) -> Self {
        Self { data, _phantom: std::marker::PhantomData }
    }

    pub fn prepare<F>(&self, transform_fn: F) -> PreparedTestCase<D, T>
    where
        F: FnOnce(D::Input, T) -> D::Input,
        T: Default,
    {
        let input = self.data.input();
        let transformer = self.data.params();
        let transformed_input = transformer.transform(input, transform_fn);
        let output = self.data.output();
        PreparedTestCase { input: transformed_input, output }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReplayData {
    pub input: String,
    pub params: ReplayTransform,
    pub extension: ReplayExtension,
    pub output: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReplayTransform {
    pub delay: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReplayExtension {
    pub stop_info: Option<StopInfo>,
}


impl Fixture<ReplayTransform> for ReplayData {
    type Input = String;
    type Output = Vec<String>;
    type Error = ();
    type Extension = ReplayExtension;

    fn input(&self) -> Self::Input {
        self.input.clone()
    }

    fn params(&self) -> ReplayTransform {
        self.params.clone()
    }

    fn extension(&self) -> Self::Extension {
        self.extension.clone()
    }

    fn output(&self) -> Result<Self::Output, Self::Error> {
        self.output.as_ref().map_or(Err(()), |output| Ok(output.clone()))
    }
}

impl Transformer<String, Self> for ReplayTransform {
    fn transform<F>(&self, input: String, transform_fn: F) -> String
    where
        F: FnOnce(String, Self) -> String,
    {
        transform_fn(input, self.clone())
    }
}