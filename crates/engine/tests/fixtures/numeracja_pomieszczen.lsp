;;; ==========================================================================
;;; CoreCAD AutoLISP - Numeracja Pomieszczen
;;; Plik: demo/lisp/numeracja_pomieszczen.lsp
;;;
;;; Polecenia:
;;;   NUMERUJ  - interaktywne numerowanie pomieszczen przez klikanie punktow
;;;   NUM-POM  - alias do NUMERUJ
;;;
;;; Funkcje programowe:
;;;   (wstaw-numer punkt tekst wysokosc warstwa)
;;;   (numeruj-liste punkt-lista prefix start-nr h warstwa)
;;; ==========================================================================

;; Funkcja pomocnicza wstawiajaca tekst z numerem pomieszczenia
(defun wstaw-numer (pt txt h layer / dxf)
  (if (null layer) (setq layer "OPISY"))
  (if (null h) (setq h 25.0))
  (setq dxf (list
    '(0 . "TEXT")
    (cons 8 layer)
    (cons 10 pt)
    (cons 11 pt)
    (cons 40 h)
    (cons 1 txt)
    (cons 50 0.0)
    (cons 72 1)   ; HAlign::Center
    (cons 73 2)   ; VAlign::Middle
  ))
  (entmake dxf)
)

;; Funkcja numerujaca z listy punktow
(defun numeruj-liste (pt-list prefix start-nr h layer / pt nr txt count)
  (if (null prefix) (setq prefix "P."))
  (if (null start-nr) (setq start-nr 1))
  (if (null h) (setq h 25.0))
  (if (null layer) (setq layer "OPISY"))
  (setq nr start-nr)
  (setq count 0)
  (foreach pt pt-list
    (setq txt (strcat prefix (itoa nr)))
    (if (wstaw-numer pt txt h layer)
      (progn
        (princ (strcat "\nPonumerowano " txt " na pozycji: (" (rtos (car pt) 2 1) ", " (rtos (cadr pt) 2 1) ")"))
        (setq nr (1+ nr))
        (setq count (1+ count))
      )
    )
  )
  (princ (strcat "\nZakonczono numeracje. Wstawiono numerow: " (itoa count)))
  count
)

;; Polecenie interaktywne
(defun c:numeruj (/ prefix start-nr h layer pt nr txt)
  (setq prefix (getstring "\nPrefiks pomieszczenia <P.>: "))
  (if (or (null prefix) (= prefix "")) (setq prefix "P."))

  (setq start-nr (getint "\nNumer poczatkowy <1>: "))
  (if (null start-nr) (setq start-nr 1))

  ;; Dobierz domyslna wysokosc z jednostek INSUNITS (4 = mm, 6 = m)
  (setq h (if (= (getvar "INSUNITS") 6) 0.25 25.0))
  (setq h-user (getdist (strcat "\nWysokosc tekstu <" (rtos h 2 2) ">: ")))
  (if h-user (setq h h-user))

  (setq layer "OPISY")
  (setq nr start-nr)

  (princ "\nWskazuj punkty pomieszczen (Esc lub Enter aby zakonczyc):")
  (while (setq pt (getpoint (strcat "\nWskaz punkt dla " prefix (itoa nr) ": ")))
    (setq txt (strcat prefix (itoa nr)))
    (wstaw-numer pt txt h layer)
    (princ (strcat " -> wstawiono " txt))
    (setq nr (1+ nr))
  )
  (princ (strcat "\nZakonczono numerowanie. Ostatni numer: " (itoa (1- nr))))
  (princ)
)

(defun c:num-pom () (c:numeruj))

(princ "\nWczytano modul numeracji pomieszczen. Uruchom: NUMERUJ")
(princ)
