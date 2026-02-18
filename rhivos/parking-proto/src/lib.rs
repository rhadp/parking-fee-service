//! Generated gRPC/protobuf bindings for the SDV Parking Demo System.
//!
//! This crate provides Rust types and gRPC service traits generated from the
//! `.proto` files under `proto/`. All Rust services in the workspace depend
//! on this crate for shared message types and service definitions.
//!
//! # Module structure
//!
//! The module hierarchy mirrors the protobuf package names:
//!
//! - [`common`] — shared message types (`parking.common`)
//! - [`services::update`] — UpdateService gRPC interface (`parking.services.update`)
//! - [`services::adapter`] — ParkingAdapter gRPC interface (`parking.services.adapter`)

/// Shared message types from `proto/common/common.proto`.
pub mod common {
    tonic::include_proto!("parking.common");
}

/// Service gRPC interfaces.
pub mod services {
    /// UpdateService gRPC interface from `proto/services/update_service.proto`.
    pub mod update {
        tonic::include_proto!("parking.services.update");
    }

    /// ParkingAdapter gRPC interface from `proto/services/parking_adapter.proto`.
    pub mod adapter {
        tonic::include_proto!("parking.services.adapter");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_types_are_accessible() {
        // Verify shared message types from common.proto are generated.
        let location = common::Location {
            latitude: 48.8566,
            longitude: 2.3522,
        };
        assert!((location.latitude - 48.8566).abs() < f64::EPSILON);

        let vehicle_id = common::VehicleId {
            vin: "WBA12345678901234".into(),
        };
        assert_eq!(vehicle_id.vin, "WBA12345678901234");

        let adapter_info = common::AdapterInfo {
            adapter_id: "test-adapter".into(),
            name: "Test Adapter".into(),
            image_ref: "registry.example.com/adapter:v1".into(),
            checksum: "sha256:abc123".into(),
            version: "1.0.0".into(),
        };
        assert_eq!(adapter_info.adapter_id, "test-adapter");

        // Verify AdapterState enum values.
        assert_eq!(common::AdapterState::Unknown as i32, 0);
        assert_eq!(common::AdapterState::Running as i32, 3);
        assert_eq!(common::AdapterState::Error as i32, 5);

        let error_details = common::ErrorDetails {
            code: "NOT_FOUND".into(),
            message: "adapter not found".into(),
            details: Default::default(),
            timestamp: 1700000000,
        };
        assert_eq!(error_details.code, "NOT_FOUND");
    }

    #[test]
    fn update_service_types_are_accessible() {
        // Verify UpdateService request/response types are generated.
        let req = services::update::InstallAdapterRequest {
            image_ref: "registry.example.com/adapter:v1".into(),
            checksum: "sha256:abc123".into(),
        };
        assert_eq!(req.image_ref, "registry.example.com/adapter:v1");

        let resp = services::update::InstallAdapterResponse {
            job_id: "job-1".into(),
            adapter_id: "adapter-1".into(),
            state: common::AdapterState::Downloading as i32,
        };
        assert_eq!(resp.state, 1);

        let _list_req = services::update::ListAdaptersRequest {};
        let _remove_req = services::update::RemoveAdapterRequest {
            adapter_id: "adapter-1".into(),
        };
        let _status_req = services::update::GetAdapterStatusRequest {
            adapter_id: "adapter-1".into(),
        };
        let _watch_req = services::update::WatchAdapterStatesRequest {};
    }

    #[test]
    fn parking_adapter_types_are_accessible() {
        // Verify ParkingAdapter request/response types are generated.
        let req = services::adapter::StartSessionRequest {
            vehicle_id: Some(common::VehicleId {
                vin: "WBA12345678901234".into(),
            }),
            zone_id: "zone-a".into(),
            timestamp: 1700000000,
        };
        assert_eq!(req.zone_id, "zone-a");

        let _stop_req = services::adapter::StopSessionRequest {
            session_id: "session-1".into(),
            timestamp: 1700001000,
        };
        let _status_req = services::adapter::GetStatusRequest {
            session_id: "session-1".into(),
        };
        let _rate_req = services::adapter::GetRateRequest {
            zone_id: "zone-a".into(),
        };
        let rate_resp = services::adapter::GetRateResponse {
            zone_id: "zone-a".into(),
            rate_per_hour: 2.50,
            currency: "EUR".into(),
        };
        assert!((rate_resp.rate_per_hour - 2.50).abs() < f64::EPSILON);
    }

    #[test]
    fn update_service_trait_is_generated() {
        // Verify the gRPC server trait exists (compile-time check).
        // We don't implement it here — just confirm the type is accessible.
        fn _assert_trait_exists<T: services::update::update_service_server::UpdateService>() {}
    }

    #[test]
    fn parking_adapter_trait_is_generated() {
        // Verify the gRPC server trait exists (compile-time check).
        fn _assert_trait_exists<T: services::adapter::parking_adapter_server::ParkingAdapter>() {}
    }
}
