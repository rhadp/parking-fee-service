package model

type Command struct {
	CommandID string   `json:"command_id"`
	Type      string   `json:"type"`
	Doors     []string `json:"doors"`
}

type CommandResponse struct {
	CommandID string `json:"command_id"`
	Status    string `json:"status"`
	Reason    string `json:"reason,omitempty"`
}
