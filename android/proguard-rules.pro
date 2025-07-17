# Add project specific ProGuard rules here.
# You can control the set of applied configuration files using the
# proguardFiles setting in build.gradle.

# Keep UniFFI generated classes
-keep class uniffi.** { *; }
-keep class com.navia.** { *; }