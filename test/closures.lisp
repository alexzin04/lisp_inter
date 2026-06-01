(define (make-adder x)
  (lambda (y) (+ x y)))

(define add-five (make-adder 5))

(add-five 10)
