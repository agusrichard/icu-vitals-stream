package producer

import (
	"encoding/binary"
	"encoding/json"
	"errors"

	"github.com/IBM/sarama"
	"github.com/agusrichard/icu-vitals-stream/simulator/internal"
	"github.com/riferrei/srclient"
)

const topic = "vitals.raw"

type Producer interface {
	Send(payload internal.VitalSigns) error
	Close() error
}

type kafkaProducer struct {
	ap     sarama.AsyncProducer
	schema *srclient.Schema
}

func NewKafkaProducer(brokers []string, schemaRegistryURL string) (Producer, error) {
	cfg := sarama.NewConfig()
	cfg.Producer.Return.Successes = false
	cfg.Producer.Return.Errors = true

	ap, err := sarama.NewAsyncProducer(brokers, cfg)
	if err != nil {
		return nil, err
	}

	srClient := srclient.NewSchemaRegistryClient(schemaRegistryURL)
	schema, err := srClient.GetLatestSchema("vitals.raw-value")
	if err != nil {
		return nil, errors.Join(err, ap.Close())
	}

	p := &kafkaProducer{ap: ap, schema: schema}
	go p.drainErrors()
	return p, nil
}

func (p *kafkaProducer) Send(payload internal.VitalSigns) error {
	b, err := json.Marshal(payload)
	if err != nil {
		return err
	}

	var native map[string]interface{}
	if err := json.Unmarshal(b, &native); err != nil {
		return err
	}
	native["timestamp"] = payload.Timestamp

	avroBytes, err := p.schema.Codec().BinaryFromNative(nil, native)
	if err != nil {
		return err
	}

	msg := make([]byte, 5+len(avroBytes))
	msg[0] = 0x00
	binary.BigEndian.PutUint32(msg[1:5], uint32(p.schema.ID()))
	copy(msg[5:], avroBytes)

	p.ap.Input() <- &sarama.ProducerMessage{
		Topic: topic,
		Key:   sarama.StringEncoder(payload.PatientID),
		Value: sarama.ByteEncoder(msg),
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
