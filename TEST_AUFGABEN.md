# DVB Dresden - MCP Agent Test-Aufgaben

Hier ist eine Sammlung von typischen Anfragen, die ein Reisender in Dresden an den MCP-Agent stellen würde:

## 🚏 Haltestellen finden

1. **Wo ist der Hauptbahnhof?**
   - Finde die Haltestelle "Hauptbahnhof"

2. **Welche Haltestellen gibt es am Postplatz?**
   - Suche nach allen Haltestellen in der Nähe von "Postplatz"

3. **Ich bin am Albertplatz, wo kann ich hier einsteigen?**
   - Finde Haltestellen in der Nähe von "Albertplatz"

## 🚌 Abfahrtszeiten

4. **Wann fährt die nächste Straßenbahn vom Hauptbahnhof?**
   - Zeige Abfahrten vom Hauptbahnhof an

5. **Welche Busse fahren in den nächsten 30 Minuten von der Prager Straße?**
   - Monitore Abfahrten von "Prager Straße" mit Limit

6. **Zeig mir alle Straßenbahnen am Postplatz**
   - Filtere Abfahrten nach Verkehrsmitteltyp (Straßenbahn)

## 🗺️ Routenplanung

7. **Wie komme ich vom Hauptbahnhof zur Frauenkirche?**
   - Route vom Hauptbahnhof zur Frauenkirche

8. **Ich möchte vom Albertplatz zum Blauen Wunder fahren**
   - Verbindungssuche mit zwei Haltestellen

9. **Wie komme ich von der TU Dresden zum Großen Garten?**
   - Route zwischen zwei bekannten Orten

10. **Ich muss um 14:30 am Bahnhof Neustadt sein, wann muss ich vom Striesen Platz losfahren?**
    - Route mit Ankunftszeit

## 📍 Sehenswürdigkeiten & POIs

11. **Wo ist die Frauenkirche?**
    - Finde den POI "Frauenkirche"

12. **Wie komme ich zum Zwinger?**
    - Finde POI und erstelle Route

13. **Zeig mir Sehenswürdigkeiten in der Altstadt**
    - Suche nach POIs in einem Gebiet

## 🎫 Linien & Details

14. **Welche Linien halten am Pirnaischen Platz?**
    - Liste alle Linien einer Haltestelle auf

15. **Zeig mir Details zur Linie 11**
    - Hole Trip-Details für eine bestimmte Linie

16. **Was ist die Stop-ID vom Hauptbahnhof?**
    - Lookup der Stop-ID

## 🧭 Kontext & Navigation

17. **Ich bin gerade am Neumarkt**
    - Setze aktuellen Standort

18. **Mein Hotel ist am Bahnhof Neustadt**
    - Setze Ausgangspunkt/Ziel

19. **Wo bin ich gerade und wo will ich hin?**
    - Zeige User-Context an

20. **Lösche meinen gespeicherten Standort**
    - Reset des User-Context

## 🕐 Zeit & Planung

21. **Wie spät ist es jetzt?**
    - Aktuelle Zeit abfragen

22. **Ich möchte morgen früh um 8 Uhr vom Hauptbahnhof zum Flughafen**
    - Route mit zukünftiger Zeit

23. **Welche Verbindungen gibt es am späten Abend von Pieschen nach Löbtau?**
    - Nachtverbindungen suchen

## 🔄 Kombinierte Anfragen

24. **Ich bin am Hauptbahnhof und möchte zur Semperoper. Wann fährt die nächste Bahn?**
    - Kontext setzen + Route + Abfahrten

25. **Finde die Kreuzkirche, zeig mir wie ich dahin komme und wann die nächste Bahn fährt**
    - POI suchen + Route + Monitoring

26. **Von meinem aktuellen Standort zur TU Dresden, aber ich möchte über den Hauptbahnhof fahren**
    - Route mit Via-Punkt

## 🌐 Karten & Links

27. **Zeig mir den Hauptbahnhof auf einer Karte**
    - OSM-Link für eine Haltestelle generieren

28. **Wo genau ist die Haltestelle Postplatz auf der Karte?**
    - Koordinaten und Kartenlink abrufen

## 🎯 Praktische Szenarien

29. **Ich komme am Hauptbahnhof an und muss zum Hotel Taschenbergpalais**
    - Komplette Reiseplanung vom Bahnhof

30. **Wie komme ich vom Flughafen Dresden in die Innenstadt?**
    - Typische Touristen-Anfrage

31. **Ich will Shoppen gehen an der Prager Straße, wie komme ich hin?**
    - Alltagsszenario

32. **Gibt es eine direkte Verbindung von Löbtau nach Klotzsche?**
    - Spezifische Verbindungsanfrage

33. **Welche Straßenbahnlinien fahren durch die Altstadt?**
    - Gebietsbezogene Linieninformation

34. **Ich habe nur 5 Minuten zum Umsteigen, schaffe ich das am Postplatz?**
    - Umsteigezeit-Anfrage

35. **Was ist der schnellste Weg vom Uniklinikum zum Hauptbahnhof?**
    - Optimierte Routensuche

---

## 💡 Tipps für Tests

- **Tippfehler einbauen**: "Haptbahnhof" statt "Hauptbahnhof"
- **Umgangssprache**: "Hauptbahni", "Neustadt Bahnhof"
- **Unvollständige Infos**: "zum Bahnhof" (welcher?)
- **Komplexe Anfragen**: Mehrere Schritte in einer Frage
- **Kontext nutzen**: "von hier nach dort" ohne explizite Orte