package producer

import "github.com/agusrichard/icu-vitals-stream/simulator/internal"

type noopProducer struct{}

func NewNoopProducer() Producer {
	return &noopProducer{}
}

func (n *noopProducer) Send(internal.VitalSigns) error { return nil }
func (n *noopProducer) Close() error                   { return nil }
