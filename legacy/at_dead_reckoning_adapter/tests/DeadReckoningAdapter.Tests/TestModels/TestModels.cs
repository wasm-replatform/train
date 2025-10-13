namespace DeadReckoningAdapter.Tests.TestModels
{
    internal class TestClass
    {
        public string? Property1 { get; set; }
        public string? Property2 { get; set; }
        public int? Property3 { get; set; }
        public string? ConfluentEnvPrefix { get; set; }

        public TestClass2? TestObject { get; set; } = null;
    }

    internal class TestClass2
    {
        public string Value { get; set; } = null!;
        public string Name { get; set; } = null!;
        public string? GroupPrefix { get; set; }
    }

    public class TestMessage
    {
        public string Content { get; set; } = null!;
    }

    public class TestObject
    {
        public string Value { get; set; } = null!;
        public string Name { get; set; } = null!;
    }
}
