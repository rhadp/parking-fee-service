package main

import "net/http"

// HandleStartParking handles POST /parking/start.
func HandleStartParking(store *SessionStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// TODO: implement
		http.Error(w, "not implemented", http.StatusNotImplemented)
	}
}

// HandleStopParking handles POST /parking/stop.
func HandleStopParking(store *SessionStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// TODO: implement
		http.Error(w, "not implemented", http.StatusNotImplemented)
	}
}

// HandleParkingStatus handles GET /parking/status.
func HandleParkingStatus(store *SessionStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// TODO: implement
		http.Error(w, "not implemented", http.StatusNotImplemented)
	}
}
