using System.Reflection;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace Takumi.Render.UniFFI;

internal static class NativeLibraryBootstrap
{
    private const string NativeLibraryName = "takumi_render_uniffi";
    private static readonly Lazy<IntPtr> NativeHandle = new(LoadNativeLibrary, isThreadSafe: true);

    [ModuleInitializer]
    internal static void Initialize()
    {
        NativeLibrary.SetDllImportResolver(typeof(NativeLibraryBootstrap).Assembly, ResolveLibrary);
    }

    private static IntPtr ResolveLibrary(string libraryName, Assembly assembly, DllImportSearchPath? searchPath)
    {
        if (!string.Equals(libraryName, NativeLibraryName, StringComparison.Ordinal))
        {
            return IntPtr.Zero;
        }

        return NativeHandle.Value;
    }

    private static IntPtr LoadNativeLibrary()
    {
        var assembly = typeof(NativeLibraryBootstrap).Assembly;
        var rid = CurrentRuntimeId();
        var fileName = NativeFileName();
        var resourceName = ResolveResourceName(assembly, rid, fileName);
        var extractionPath = ExtractionPath(assembly, rid, fileName);

        Directory.CreateDirectory(extractionPath.Directory!.FullName);

        using (var resourceStream = assembly.GetManifestResourceStream(resourceName)
               ?? throw new FileNotFoundException($"Embedded native resource '{resourceName}' was not found."))
        using (var fileStream = new FileStream(extractionPath.FullName, FileMode.Create, FileAccess.Write, FileShare.Read))
        {
            resourceStream.CopyTo(fileStream);
        }

        return NativeLibrary.Load(extractionPath.FullName);
    }

    private static string ResolveResourceName(Assembly assembly, string rid, string fileName)
    {
        var ridVariants = new[] { rid, rid.Replace('-', '_') };
        var resourceName = assembly
            .GetManifestResourceNames()
            .SingleOrDefault(name => ridVariants.Any(
                variant => name.EndsWith($".Generated.Native.{variant}.{fileName}", StringComparison.Ordinal)));

        return resourceName
            ?? throw new FileNotFoundException(
                $"Unable to locate embedded native resource for RID '{rid}' and file '{fileName}'.");
    }

    private static FileInfo ExtractionPath(Assembly assembly, string rid, string fileName)
    {
        var version = assembly.GetName().Version?.ToString() ?? "0.1.0";
        var root = Path.Combine(Path.GetTempPath(), "Takumi.Render.UniFFI", version, rid);
        return new FileInfo(Path.Combine(root, fileName));
    }

    private static string CurrentRuntimeId()
    {
        var os = RuntimeInformation.IsOSPlatform(OSPlatform.Windows)
            ? "win"
            : RuntimeInformation.IsOSPlatform(OSPlatform.OSX)
                ? "osx"
                : RuntimeInformation.IsOSPlatform(OSPlatform.Linux)
                    ? "linux"
                    : throw new PlatformNotSupportedException("Unsupported OS platform.");

        var architecture = RuntimeInformation.ProcessArchitecture switch
        {
            Architecture.X64 => "x64",
            Architecture.Arm64 => "arm64",
            _ => throw new PlatformNotSupportedException(
                $"Unsupported CPU architecture: {RuntimeInformation.ProcessArchitecture}")
        };

        return $"{os}-{architecture}";
    }

    private static string NativeFileName()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            return $"{NativeLibraryName}.dll";
        }

        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
        {
            return $"lib{NativeLibraryName}.dylib";
        }

        if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
        {
            return $"lib{NativeLibraryName}.so";
        }

        throw new PlatformNotSupportedException("Unsupported OS platform.");
    }
}