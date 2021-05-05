package executionThreads;

import java.time.LocalDateTime;
import java.time.format.DateTimeFormatter;

import constants.CurrentConfiguration;

 public class Timer {
	DateTimeFormatter dtf = DateTimeFormatter.ofPattern("dd/MM/yyyy HH:mm:ss"); 
	LocalDateTime now;
	LocalDateTime later;
	
	public boolean startTimer() {
		now = LocalDateTime.now();
		later = now.plusSeconds(CurrentConfiguration.buttonDisableDuration);
		while( ! now.isAfter(later)) {
			try {
				Thread.sleep(1000);
				now = LocalDateTime.now();
			} catch (InterruptedException e) {
				e.printStackTrace();
			}
		}
		return true;
	}
}

