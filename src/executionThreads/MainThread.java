package executionThreads;

import constants.CurrentConfiguration;

public class MainThread{	
	public static void main(String[] args) {
		CurrentConfiguration.setDarkMode();
			Hourglass hourglass = new Hourglass();
			System.out.println("Starting threads.");
			try {
				hourglass.runThreads();
			} catch (Exception e) {
				e.printStackTrace();
			}
	}
}

class Hourglass {
	private Lock lock = new Lock();
	TimerThread timerThread = new TimerThread(lock);
	PopupThread popupThread = new PopupThread(lock);
	
	private void runTimer() throws Exception{
		System.out.println("Timer Thread holds starter lock");
		timerThread.start();
	}
	private void openPopup() throws Exception {
		System.out.println("Popup Thread holds starter lock");
		popupThread.start();
	}
	//This function starts the timer thread and popupWindow thread
	public void runThreads() throws Exception {
		this.runTimer();
		this.openPopup();
		System.out.println("Started Threads");
	}
	
}