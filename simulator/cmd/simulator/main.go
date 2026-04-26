package main

import (
	"context"
	"flag"
	"log"
	"os"
	"os/signal"
	"strings"
	"sync"
	"syscall"

	"github.com/agusrichard/icu-vitals-stream/simulator/internal"
	"github.com/agusrichard/icu-vitals-stream/simulator/producer"
)

func main() {
	patients := flag.Int("patients", 1, "Number of patients to simulate")
	brokers := flag.String("brokers", "localhost:9092", "Kafka broker list")
	noKafka := flag.Bool("no-kafka", false, "Discard Kafka messages (use when broker is not running)")
	flag.Parse()
	internal.LogInfo("patients=%d, brokers=%s, no-kafka=%t\n",
		*patients, *brokers, *noKafka)

	var p producer.Producer
	if *noKafka {
		p = producer.NewNoopProducer()
	} else {
		var err error
		p, err = producer.NewKafkaProducer(strings.Split(*brokers, ","))
		if err != nil {
			log.Fatal(err)
		}
	}
	ctx, stop := signal.NotifyContext(context.Background(), syscall.SIGTERM, os.Interrupt)
	defer stop()

	var wg sync.WaitGroup
	for i := 0; i < *patients; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			NewPatient(p).Run(ctx)
		}()
	}

	wg.Wait()
	if err := p.Close(); err != nil {
		internal.LogErr(err, "producer close")
	}
}
