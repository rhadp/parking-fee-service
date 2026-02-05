# ProGuard rules for SDV Parking App

# Keep gRPC classes
-keep class io.grpc.** { *; }
-keepclassmembers class io.grpc.** { *; }

# Keep Protocol Buffer generated classes
-keep class com.google.protobuf.** { *; }
-keepclassmembers class com.google.protobuf.** { *; }

# Keep generated proto classes
-keep class com.sdv.** { *; }
-keepclassmembers class com.sdv.** { *; }

# Keep Kotlin coroutines
-keepclassmembers class kotlinx.coroutines.** { *; }

# Keep Retrofit
-keepattributes Signature
-keepattributes Exceptions
-keep class retrofit2.** { *; }
-keepclasseswithmembers class * {
    @retrofit2.http.* <methods>;
}

# Keep OkHttp
-dontwarn okhttp3.**
-dontwarn okio.**
-keep class okhttp3.** { *; }
-keep interface okhttp3.** { *; }

# Keep Gson
-keep class com.google.gson.** { *; }
-keepattributes *Annotation*

# General Android rules
-keepattributes SourceFile,LineNumberTable
-renamesourcefileattribute SourceFile
