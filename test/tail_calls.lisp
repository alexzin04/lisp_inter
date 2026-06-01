(define (sum-list lst acc)
  (if (null? lst)
      acc
      (sum-list (cdr lst) (+ acc (car lst)))))

(sum-list '(1 2 3 4 5) 0)
