package dev.nl.mirror.input

import android.content.Context
import android.location.Location
import android.location.LocationManager
import android.location.provider.ProviderProperties
import android.os.Build
import android.os.SystemClock
import dev.nl.mirror.util.FakeContext

object LocationController {
    private val locationManager: LocationManager by lazy {
        FakeContext.get().getSystemService(Context.LOCATION_SERVICE) as LocationManager
    }
    
    private val providers = listOf(
        LocationManager.GPS_PROVIDER,
        LocationManager.NETWORK_PROVIDER, 
        LocationManager.FUSED_PROVIDER
    )
    
    private var isMocking = false

    fun startMocking() {
        if (isMocking) return
        
        try {
            providers.forEach { provider ->
                try {
                    locationManager.addTestProvider(
                        provider,
                        false, // requiresNetwork
                        false, // requiresSatellite
                        false, // requiresCell
                        false, // hasMonetaryCost
                        true,  // supportsAltitude
                        true,  // supportsSpeed
                        true,  // supportsBearing
                        ProviderProperties.POWER_USAGE_LOW,
                        ProviderProperties.ACCURACY_FINE
                    )
                } catch (e: IllegalArgumentException) {
                    // Provider might already exist
                } catch (e: SecurityException) {
                   // Ignore if we lack permission (should be granted by shell)
                }
                
                try {
                    locationManager.setTestProviderEnabled(provider, true)
                } catch (e: Exception) {
                }
            }
            isMocking = true
        } catch (e: Exception) {
            e.printStackTrace()
            // Continue best effort
        }
    }

    fun stopMocking() {
        if (!isMocking) return
        
        providers.forEach { provider ->
            try {
                locationManager.removeTestProvider(provider)
            } catch (e: Exception) {
                // Ignore
            }
        }
        isMocking = false
    }

    fun updateLocation(lat: Double, lon: Double, alt: Double = 0.0, bearing: Float = 0.0f, speed: Float = 0.0f) {
        if (!isMocking) {
            startMocking()
        }

        providers.forEach { provider ->
            val loc = Location(provider)
            loc.latitude = lat
            loc.longitude = lon
            loc.altitude = alt
            loc.bearing = bearing
            loc.speed = speed
            loc.accuracy = 5.0f
            loc.time = System.currentTimeMillis()
            
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.JELLY_BEAN_MR1) {
                loc.elapsedRealtimeNanos = SystemClock.elapsedRealtimeNanos()
            }
            
            // Required for Android 12+ (S) to make location "complete"
            if (Build.VERSION.SDK_INT >= 34) { // U+ (API 34)
                // Reflective call currently not needed if setTestProviderLocation handles it,
                // but usually making it 'complete' is safer if API allows.
                // However, LocationBuilder is the modern way, but we are keeping it simple for now.
                // 'makeComplete' is hidden API.
            }

            try {
                 locationManager.setTestProviderLocation(provider, loc)
            } catch (e: Exception) {
                 e.printStackTrace()
            }
        }
    }
}
