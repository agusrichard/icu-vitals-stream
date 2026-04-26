package producer

import (
	"encoding/json"

	"github.com/IBM/sarama"
	"github.com/agusrichard/icu-vitals-stream/simulator/internal"
)

const topic = "vitals.raw"

type Producer interface {
	Send(payload internal.VitalSigns) error
	Close() error
}

type kafkaProducer struct {
	ap sarama.AsyncProducer
}

func NewKafkaProducer(brokers []string) (Producer, error) {
	cfg := sarama.NewConfig()
	cfg.Producer.Return.Successes = false
	cfg.Producer.Return.Errors = true

	ap, err := sarama.NewAsyncProducer(brokers, cfg)
	if err != nil {
		return nil, err
	}

	p := &kafkaProducer{ap: ap}
	go p.drainErrors()
	return p, nil
}

func (p *kafkaProducer) Send(payload internal.VitalSigns) error {
	b, err := json.Marshal(payload)
	if err != nil {
		return err
	}

	p.ap.Input() <- &sarama.ProducerMessage{
		Topic: topic,
		Key:   sarama.StringEncoder(payload.PatientID),
		Value: sarama.ByteEncoder(b),
	}

	return nil
}

func (p *kafkaProducer) Close() error {
	return p.ap.Close()
}

func (p *kafkaProducer) drainErrors() {
	for err := range p.ap.Errors() {
		_ = err
	}
}
