Pod::Spec.new do |spec|
  spec.name          = "Navia"
  spec.version       = "1.2.8"
  spec.summary       = "Secure DIDComm v2 messaging library for iOS"
  spec.description   = <<-DESC
                       Navia is a secure DIDComm v2 messaging library for mobile applications,
                       built in Rust with native bindings for iOS through UniFFI. The library
                       uses Aries Askar for secure storage and implements the full DIDComm
                       protocol with encryption and authentication.
                       DESC

  spec.homepage      = "https://github.com/Nyx-Chat/navia"
  spec.license       = { :type => "Proprietary", :text => "Proprietary License" }
  spec.author        = { "Nyx Development Team" => "team@nyx.chat" }

  spec.ios.deployment_target = "13.0"
  spec.swift_version = "5.0"

  spec.source        = { :git => "https://github.com/Nyx-Chat/navia.git", :tag => "v#{spec.version}" }

  spec.source_files  = "ios/Navia/**/*.{h,swift}"
  spec.public_header_files = "ios/Navia/Navia.h", "ios/Navia/Generated/navia_coreFFI.h"

  # System frameworks required by the Rust library
  spec.frameworks = "Foundation", "Security"

  # System libraries required by Rust dependencies
  spec.libraries = "c++", "resolv"

  # Compiler flags
  spec.pod_target_xcconfig = {
    'LIBRARY_SEARCH_PATHS' => '$(SRCROOT)/Navia/rust/target/universal/release $(SRCROOT)/Navia/rust/target/universal/debug',
    'OTHER_LDFLAGS' => '-lnavia_core',
    'ENABLE_USER_SCRIPT_SANDBOXING' => 'NO'
  }

  # Build script to compile Rust library
  spec.script_phase = {
    :name => 'Build Rust Library',
    :script => <<-SCRIPT,
      set -e
      cd "${PODS_TARGET_SRCROOT}/scripts"
      ./build-for-ios.sh --release --package
SCRIPT
    :execution_position => :before_compile
  }

  # Exclude generated files from source control (they're built dynamically)
  spec.preserve_paths = "rust/**/*", "scripts/build-for-ios.sh"

  spec.prepare_command = <<-CMD
    # Ensure the build script is executable
    chmod +x scripts/build-for-ios.sh

    # Create Generated directory if it doesn't exist
    mkdir -p ios/Navia/Generated
  CMD
end
