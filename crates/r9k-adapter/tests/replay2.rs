//! Tests for expected success and failure outputs from the R9k adapter for a
//! set of inputs captured as snapshots from the live system.

mod provider;

use std::fs::{self, File};

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
// constructing (say by deserialization) an input and an output. Optionally, a
// transform function can be provided to modify the input before processing. The
// transform function can also take parameters, which can be provided by the
// generic parameter. This defaults to the unit type if not specified.
pub trait Fixture<P = ()> {
    type Input;
    type Output;
    type Error;

    // Transform input data into the input type needed by the test case handler.
    fn input(&self) -> Self::Input;

    // Transform input data into transformation parameters for the test case
    // handler.
    fn params(&self) -> P
    where
        P: Default,
    {
        P::default()
    }

    fn transform<F>(&self, input: Self::Input, _: P, _: F) -> Self::Input
    where
        F: FnOnce(Self::Input, P) -> Self::Input,
    {
        input
    }

    /// Transform input data into the expected output type needed by the test
    /// case handler, which could be an error for failure cases.
    ///
    /// # Errors
    ///
    /// Returns an error when the fixture cannot produce the expected output.
    fn output(&self) -> Result<Self::Output, Self::Error>;
}

pub struct TestCase<D, P> {
    data: D,
    _phantom: std::marker::PhantomData<P>,
}

pub struct PreparedTestCase<D, P = ()>
where
    D: Fixture<P>,
{
    pub input: D::Input,
    pub output: Result<D::Output, D::Error>,
}

impl<D, P> TestCase<D, P>
where
    D: Clone + Fixture<P>,
{
    #[must_use]
    pub const fn new(data: D) -> Self {
        Self { data, _phantom: std::marker::PhantomData }
    }

    pub fn prepare<F>(&self, transform: F) -> PreparedTestCase<D, P>
    where
        F: FnOnce(D::Input, P) -> D::Input,
        P: Default,
    {
        let input = self.data.input();
        let transformed_input = self.data.transform(input, self.data.params(), transform);
        let output = self.data.output();
        PreparedTestCase { input: transformed_input, output }
    }
}
