namespace DeadReckoningAdapter.Core.Exceptions;

public class NoConfigurationException : Exception
{
    public NoConfigurationException(string? message) : base(message)
    {
    }
}
