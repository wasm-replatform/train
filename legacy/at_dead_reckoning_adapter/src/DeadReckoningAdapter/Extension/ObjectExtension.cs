using System.Collections;

namespace DeadReckoningAdapter.Extension
{
    public static class ObjectExtension
    {
        public static bool AllPropertiesNotNullOrEmpty(this object obj)
        {
            if (obj == null)
                return false;

            foreach (var property in obj.GetType().GetProperties())
            {
                if (!property.CanRead)
                    continue;

                var value = property.GetValue(obj);
                if (value == null)
                    return false;

                if (value is string stringValue && string.IsNullOrEmpty(stringValue) && !property.Name.Contains("Prefix"))
                {
                    return false;
                }

                if (!property.PropertyType.IsPrimitive &&
                    property.PropertyType != typeof(string) &&
                    !typeof(IEnumerable).IsAssignableFrom(property.PropertyType) &&
                    !AllPropertiesNotNullOrEmpty(value))
                {
                    return false;
                }
            }

            return true;
        }
    }
}
